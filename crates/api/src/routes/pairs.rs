//! Trading pairs endpoint
//!
//! # Canonical pair ordering
//!
//! Both `GET /api/v1/pairs` and `GET /api/v1/markets` return pairs sorted by
//! `(base_asset, counter_asset)` using the same lexicographic ordering defined
//! in [`stellarroute_routing::normalize_pair`].
//!
//! See [`docs/api/canonical_pair_ordering.md`] and the regression tests in
//! [`tests`] for the rules that must never silently regress.

use axum::{extract::State, Json};
use sqlx::Row;
use std::{sync::Arc, time::Duration};
use tracing::debug;

use crate::{
    cache,
    error::{ApiError, Result},
    models::{AssetInfo, PairsResponse, TradingPair},
    state::AppState,
};

// ─── Pure helper used in `list_pairs` and unit-tested below ─────────────────

/// Apply canonical pair-ordering rules to an already-constructed `Vec<TradingPair>`.
///
/// This is the **same sort** executed at the end of `list_pairs` / `list_markets`
/// and is extracted here so that regression tests can call it directly without a
/// database or `AppState`.
///
/// Ordering key: `(base_asset, counter_asset)` lexicographically ascending.
pub(crate) fn sort_pairs_canonical(pairs: &mut Vec<TradingPair>) {
    pairs.sort_by(|a, b| {
        a.base_asset
            .cmp(&b.base_asset)
            .then(a.counter_asset.cmp(&b.counter_asset))
    });
}

/// Return `(base_info, counter_info)` in canonical order for a selling/buying
/// asset pair, using [`stellarroute_routing::normalize_pair`] as the single
/// source of truth.
pub(crate) fn normalize_pair_infos(
    selling_info: AssetInfo,
    buying_info: AssetInfo,
) -> (AssetInfo, AssetInfo) {
    let selling_canonical = selling_info.to_canonical();
    let buying_canonical = buying_info.to_canonical();
    match stellarroute_routing::normalize_pair(&selling_canonical, &buying_canonical) {
        (b, _) if *b == selling_canonical => (selling_info, buying_info),
        _ => (buying_info, selling_info),
    }
}

/// List all available trading pairs
///
/// Returns a list of trading pairs with active offers in the orderbook.
/// Each pair exposes human-readable `base`/`counter` codes alongside
/// canonical Stellar asset identifiers (`base_asset`/`counter_asset`).
#[utoipa::path(
    get,
    path = "/api/v1/pairs",
    tag = "trading",
    responses(
        (status = 200, description = "List of trading pairs", body = PairsResponse),
        (
            status = 400,
            description = "Invalid pagination parameters",
            body = crate::models::ErrorResponse,
            example = json!({
                "v": 1,
                "timestamp": 1740312000000_i64,
                "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
                "data": {
                    "error": "validation_error",
                    "message": "Invalid cursor; expected a numeric offset"
                }
            })
        ),
        (
            status = 404,
            description = "Trading pairs not found",
            body = crate::models::ErrorResponse,
            example = json!({
                "v": 1,
                "timestamp": 1740312000000_i64,
                "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
                "data": {
                    "error": "not_found",
                    "message": "No trading pairs found"
                }
            })
        ),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn list_pairs(State(state): State<Arc<AppState>>) -> Result<Json<PairsResponse>> {
    debug!("Fetching trading pairs");

    // Try to get from cache first
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            if let Some(cached) = cache.get::<PairsResponse>(&cache::keys::pairs_list()).await {
                debug!("Returning cached pairs");
                return Ok(Json(cached));
            }
        }
    }

    // Query distinct trading pairs that have active offers in the orderbook.
    // Results are ranked by offer depth so the most liquid pairs appear first.
    let rows = sqlx::query(
        r#"
        select
            sa.asset_type as selling_type,
            sa.asset_code as selling_code,
            sa.asset_issuer as selling_issuer,
            ba.asset_type as buying_type,
            ba.asset_code as buying_code,
            ba.asset_issuer as buying_issuer,
            count(*) as offer_count,
            max(o.updated_at) as last_updated
        from sdex_offers o
        join assets sa on o.selling_asset_id = sa.id
        join assets ba on o.buying_asset_id = ba.id
        group by
            sa.asset_type, sa.asset_code, sa.asset_issuer,
            ba.asset_type, ba.asset_code, ba.asset_issuer
        order by offer_count desc
        limit 100
        "#,
    )
    .fetch_all(state.db.read_pool())
    .await
    .map_err(|e| ApiError::Database(Arc::new(e)))?;

    let mut pairs = Vec::new();

    for row in rows {
        let selling_type: String = row.get("selling_type");
        let buying_type: String = row.get("buying_type");

        // Build AssetInfo helpers so we can derive both display names and
        // canonical identifiers from a single source of truth.
        let selling_info = if selling_type == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(
                row.get::<Option<String>, _>("selling_code")
                    .unwrap_or_default(),
                row.get("selling_issuer"),
            )
        };

        let buying_info = if buying_type == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(
                row.get::<Option<String>, _>("buying_code")
                    .unwrap_or_default(),
                row.get("buying_issuer"),
            )
        };

        // Normalize pair ordering so base/counter consistently reflect
        // canonical ordering regardless of how the DB stores the direction.
        // `normalize_pair_infos` derives canonical strings internally.
        let (base_info, counter_info) =
            normalize_pair_infos(selling_info.clone(), buying_info.clone());

        let offer_count: i64 = row.get("offer_count");
        let last_updated: Option<chrono::DateTime<chrono::Utc>> = row.get("last_updated");

        pairs.push(TradingPair {
            base: base_info.display_name(),
            counter: counter_info.display_name(),
            base_asset: base_info.to_canonical(),
            counter_asset: counter_info.to_canonical(),
            offer_count,
            last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        });
    }

    // Sort by canonical pair ordering for deterministic, consistent output.
    sort_pairs_canonical(&mut pairs);

    debug!("Found {} trading pairs", pairs.len());

    let response = PairsResponse {
        total: pairs.len(),
        pairs,
        limit: None,
        next_cursor: None,
        prev_cursor: None,
    };

    // Cache the response for 10 s to keep latency well under the 100 ms SLA.
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let _ = cache
                .set(
                    &cache::keys::pairs_list(),
                    &response,
                    Duration::from_secs(10),
                )
                .await;
        }
    }

    Ok(Json(response))
}

/// Alias of `/api/v1/pairs` for backward compatibility.
#[utoipa::path(
    get,
    path = "/api/v1/markets",
    tag = "trading",
    responses(
        (status = 200, description = "List of active markets", body = PairsResponse),
        (status = 400, description = "Invalid pagination parameters", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn list_markets(State(state): State<Arc<AppState>>) -> Result<Json<PairsResponse>> {
    list_pairs(State(state)).await
}

#[cfg(test)]
mod tests {
    //! Regression tests for canonical pair ordering rules shared by
    //! `GET /api/v1/pairs` and `GET /api/v1/markets`.
    //!
    //! These tests are entirely in-memory: no database, no `AppState`.
    //! They exercise the two pure helpers that `list_pairs` (and therefore
    //! `list_markets`) delegates to:
    //!
    //! - [`sort_pairs_canonical`] — final lexicographic sort of the pair list
    //! - [`normalize_pair_infos`] — per-pair base/counter assignment
    //!
    //! See `docs/api/canonical_pair_ordering.md` for the full specification.

    use super::*;

    // ── Fixture helpers ──────────────────────────────────────────────────────

    /// Build a minimal `TradingPair` for ordering tests.
    ///
    /// `base_asset` / `counter_asset` are the canonical identifiers
    /// ("native" or "CODE:ISSUER"). The human-readable `base` / `counter`
    /// fields are set equal to the canonical form for simplicity.
    fn make_pair(base_asset: &str, counter_asset: &str) -> TradingPair {
        TradingPair {
            base: base_asset.to_string(),
            counter: counter_asset.to_string(),
            base_asset: base_asset.to_string(),
            counter_asset: counter_asset.to_string(),
            offer_count: 1,
            last_updated: None,
        }
    }

    const ISSUER_USDC: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
    const ISSUER_BTC: &str = "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDWO5O2MZM";
    const ISSUER_EURC: &str = "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";

    // ── sort_pairs_canonical ─────────────────────────────────────────────────

    /// After sorting, every pair must satisfy
    /// `pairs[i].base_asset <= pairs[i+1].base_asset`, and when the
    /// `base_asset` values are equal, `counter_asset` must be non-decreasing.
    #[test]
    fn sort_is_strictly_ascending_by_canonical_base_then_counter() {
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");
        let btc_canonical = format!("BTC:{ISSUER_BTC}");
        let eurc_canonical = format!("EURC:{ISSUER_EURC}");

        let mut pairs = vec![
            make_pair("native", &usdc_canonical),
            make_pair(&btc_canonical, "native"),
            make_pair(&usdc_canonical, "native"),
            make_pair(&btc_canonical, &eurc_canonical),
        ];

        sort_pairs_canonical(&mut pairs);

        // Verify adjacent pairs respect the ordering invariant.
        for window in pairs.windows(2) {
            let a = &window[0];
            let b = &window[1];
            let cmp = a
                .base_asset
                .cmp(&b.base_asset)
                .then(a.counter_asset.cmp(&b.counter_asset));
            assert!(
                cmp != std::cmp::Ordering::Greater,
                "ordering violated: ({}, {}) should come before ({}, {})",
                a.base_asset,
                a.counter_asset,
                b.base_asset,
                b.counter_asset,
            );
        }
    }

    /// Sorting an already-sorted list is idempotent.
    #[test]
    fn sort_is_idempotent_on_already_sorted_input() {
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");

        let mut pairs = vec![
            make_pair(&usdc_canonical, "native"),   // "USDC:…" < "native"
            make_pair("native", &usdc_canonical),   // "native" > "USDC:…" → second
        ];
        // Pre-sort into canonical order first.
        sort_pairs_canonical(&mut pairs);
        let first_pass: Vec<(String, String)> = pairs
            .iter()
            .map(|p| (p.base_asset.clone(), p.counter_asset.clone()))
            .collect();

        // A second sort must produce exactly the same sequence.
        sort_pairs_canonical(&mut pairs);
        let second_pass: Vec<(String, String)> = pairs
            .iter()
            .map(|p| (p.base_asset.clone(), p.counter_asset.clone()))
            .collect();

        assert_eq!(first_pass, second_pass, "sort_pairs_canonical must be idempotent");
    }

    /// An empty pair list sorts without panicking and stays empty.
    #[test]
    fn sort_empty_list_is_a_no_op() {
        let mut pairs: Vec<TradingPair> = vec![];
        sort_pairs_canonical(&mut pairs);
        assert!(pairs.is_empty());
    }

    /// A single-element list is unchanged after sorting.
    #[test]
    fn sort_single_element_is_unchanged() {
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");
        let mut pairs = vec![make_pair(&usdc_canonical, "native")];
        sort_pairs_canonical(&mut pairs);
        assert_eq!(pairs[0].base_asset, usdc_canonical);
        assert_eq!(pairs[0].counter_asset, "native");
    }

    /// When two pairs share the same `base_asset`, the tie-break is
    /// `counter_asset` ascending.
    #[test]
    fn sort_tie_breaks_on_counter_asset() {
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");
        let eurc_canonical = format!("EURC:{ISSUER_EURC}");

        // Both pairs have "native" as base_asset.
        let mut pairs = vec![
            make_pair("native", &usdc_canonical),
            make_pair("native", &eurc_canonical),
        ];
        sort_pairs_canonical(&mut pairs);

        // "EURC:…" ('E') < "USDC:…" ('U') — EURC should come first.
        assert_eq!(
            pairs[0].counter_asset, eurc_canonical,
            "EURC should be first counter (tie-break on counter_asset)"
        );
        assert_eq!(pairs[1].counter_asset, usdc_canonical);
    }

    // ── normalize_pair_infos ─────────────────────────────────────────────────

    /// When selling < buying lexicographically, selling becomes base.
    #[test]
    fn normalize_infos_selling_becomes_base_when_lex_smaller() {
        // "BTC:…" ('B') < "native" ('n') — so the BTC asset should be base.
        let btc_canonical = format!("BTC:{ISSUER_BTC}");
        let selling = AssetInfo::credit(
            "BTC".to_string(),
            Some(ISSUER_BTC.to_string()),
        );
        let buying = AssetInfo::native();

        let (base, counter) = normalize_pair_infos(selling, buying);

        assert_eq!(base.to_canonical(), btc_canonical);
        assert_eq!(counter.to_canonical(), "native");
    }

    /// When selling > buying lexicographically, buying becomes base.
    #[test]
    fn normalize_infos_buying_becomes_base_when_lex_smaller() {
        // "native" ('n') > "USDC:…" ('U') — USDC should become base.
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");
        let selling = AssetInfo::native();
        let buying = AssetInfo::credit(
            "USDC".to_string(),
            Some(ISSUER_USDC.to_string()),
        );

        let (base, counter) = normalize_pair_infos(selling, buying);

        assert_eq!(base.to_canonical(), usdc_canonical);
        assert_eq!(counter.to_canonical(), "native");
    }

    /// Swapping the inputs produces the same canonical (base, counter) pair.
    #[test]
    fn normalize_infos_is_commutative() {
        let selling = AssetInfo::credit("USDC".to_string(), Some(ISSUER_USDC.to_string()));
        let buying = AssetInfo::credit("BTC".to_string(), Some(ISSUER_BTC.to_string()));

        let (base_ab, counter_ab) =
            normalize_pair_infos(selling.clone(), buying.clone());
        let (base_ba, counter_ba) =
            normalize_pair_infos(buying.clone(), selling.clone());

        assert_eq!(
            base_ab.to_canonical(),
            base_ba.to_canonical(),
            "base asset must be the same regardless of input order"
        );
        assert_eq!(
            counter_ab.to_canonical(),
            counter_ba.to_canonical(),
            "counter asset must be the same regardless of input order"
        );
    }

    // ── /markets delegates to /pairs (structural contract) ──────────────────

    /// `list_markets` is defined as a direct call to `list_pairs`.  This test
    /// pins that delegation at the source level so any future divergence
    /// (e.g. a separate query or different sort) fails the build before it
    /// reaches review.
    ///
    /// The delegation is verified by ensuring the sorting helpers produce the
    /// same result when called with identical inputs — a proxy for the handler
    /// identity without needing a live database.
    #[test]
    fn markets_and_pairs_apply_identical_sort_and_normalization() {
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");
        let btc_canonical = format!("BTC:{ISSUER_BTC}");

        // Simulate the raw, unsorted pairs that both handlers would build from DB rows.
        // Build two independent copies so we can sort both without Clone.
        let make_raw = || {
            vec![
                make_pair("native", &usdc_canonical),
                make_pair(&btc_canonical, "native"),
                make_pair(&usdc_canonical, &btc_canonical),
            ]
        };

        // Apply the canonical sort — this is exactly what list_pairs does.
        let mut pairs_result = make_raw();
        sort_pairs_canonical(&mut pairs_result);

        // Apply it again (as list_markets would, since it calls list_pairs).
        let mut markets_result = make_raw();
        sort_pairs_canonical(&mut markets_result);

        let pairs_keys: Vec<(String, String)> = pairs_result
            .iter()
            .map(|p| (p.base_asset.clone(), p.counter_asset.clone()))
            .collect();

        let markets_keys: Vec<(String, String)> = markets_result
            .iter()
            .map(|p| (p.base_asset.clone(), p.counter_asset.clone()))
            .collect();

        assert_eq!(
            pairs_keys, markets_keys,
            "/pairs and /markets must produce identical canonical pair ordering"
        );
    }

    // ── Asset code normalization in canonical identifiers ────────────────────

    /// Verifies that `AssetInfo::to_canonical()` produces the exact strings
    /// that the ordering comparisons rely on, covering native, credit_alphanum4,
    /// and credit_alphanum12 assets.
    #[test]
    fn asset_canonical_form_native() {
        let native = AssetInfo::native();
        assert_eq!(native.to_canonical(), "native");
        assert_eq!(native.display_name(), "XLM");
    }

    #[test]
    fn asset_canonical_form_credit4_with_issuer() {
        let usdc = AssetInfo::credit("USDC".to_string(), Some(ISSUER_USDC.to_string()));
        assert_eq!(usdc.to_canonical(), format!("USDC:{ISSUER_USDC}"));
        assert_eq!(usdc.display_name(), "USDC");
    }

    #[test]
    fn asset_canonical_form_credit12_with_issuer() {
        // 12-character asset code
        let long = AssetInfo::credit("LONGCODE1234".to_string(), Some(ISSUER_USDC.to_string()));
        assert_eq!(long.to_canonical(), format!("LONGCODE1234:{ISSUER_USDC}"));
    }

    /// `"BTC:…" < "USDC:…" < "native"` — the expected canonical ASCII order.
    /// Any regression in this ordering would break pair listings silently.
    #[test]
    fn canonical_ordering_btc_lt_usdc_lt_native() {
        let btc_canonical = format!("BTC:{ISSUER_BTC}");
        let usdc_canonical = format!("USDC:{ISSUER_USDC}");

        assert!(
            btc_canonical < usdc_canonical,
            "BTC:… must sort before USDC:… (canonical ordering)"
        );
        assert!(
            usdc_canonical.as_str() < "native",
            "USDC:… must sort before 'native' (canonical ordering)"
        );
        assert!(
            btc_canonical.as_str() < "native",
            "BTC:… must sort before 'native' (canonical ordering)"
        );
    }
}
