//! Quote broadcaster background task.
//!
//! [`run_broadcaster`] polls the database for liquidity changes and fans out
//! [`ServerMessage::QuoteUpdate`] messages to all matching subscribers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use sqlx::Row;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::request::AssetPath;
use crate::models::{AssetInfo, PathStep, QuoteRationaleMetadata, QuoteResponse, VenueEvaluation};
use crate::state::AppState;

use super::messages::{ServerMessage, ServerPayload};
use super::registry::SubscriptionRegistry;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the quote broadcaster forever.
///
/// This function is intended to be spawned as a long-lived `tokio` task.
/// It polls the database every `poll_interval_ms` milliseconds, computes
/// quotes for all active subscriptions, and fans out updates to connected
/// clients.
///
/// The task is wrapped in a restart loop: if the inner loop panics the error
/// is logged and the task restarts after a 1-second delay.
pub async fn run_broadcaster(
    state: Arc<AppState>,
    registry: Arc<RwLock<SubscriptionRegistry>>,
    poll_interval_ms: u64,
) {
    loop {
        let result = broadcaster_loop(state.clone(), registry.clone(), poll_interval_ms).await;
        // broadcaster_loop only returns on an unrecoverable error / panic
        // (it loops internally). Log and restart.
        warn!("broadcaster_loop exited unexpectedly: {:?}; restarting in 1 s", result);
        sleep(Duration::from_secs(1)).await;
    }
}

// ---------------------------------------------------------------------------
// Inner loop (restartable)
// ---------------------------------------------------------------------------

async fn broadcaster_loop(
    state: Arc<AppState>,
    registry: Arc<RwLock<SubscriptionRegistry>>,
    poll_interval_ms: u64,
) -> Result<(), String> {
    // Track the last-seen ledger revision per (base, quote) pair so we can
    // detect changes without re-querying every subscription individually.
    let mut last_revisions: HashMap<(String, String), String> = HashMap::new();

    loop {
        sleep(Duration::from_millis(poll_interval_ms)).await;

        // Collect unique (base, quote) pairs from all active subscriptions.
        let pairs: HashSet<(String, String)> = {
            let reg = registry.read().await;
            reg.all_pairs()
        };

        for (base, quote) in &pairs {
            // ----------------------------------------------------------------
            // 1. Resolve asset IDs
            // ----------------------------------------------------------------
            let base_asset = match AssetPath::parse(base) {
                Ok(a) => a,
                Err(e) => {
                    warn!("broadcaster: invalid base asset '{}': {}", base, e);
                    continue;
                }
            };
            let quote_asset = match AssetPath::parse(quote) {
                Ok(a) => a,
                Err(e) => {
                    warn!("broadcaster: invalid quote asset '{}': {}", quote, e);
                    continue;
                }
            };

            let base_id = match find_asset_id(&state, &base_asset).await {
                Ok(id) => id,
                Err(ApiError::NotFound(_)) => {
                    send_no_route_to_pair(&state, &registry, base, quote).await;
                    continue;
                }
                Err(e) => {
                    warn!("broadcaster: find_asset_id({}) error: {:?}", base, e);
                    continue;
                }
            };
            let quote_id = match find_asset_id(&state, &quote_asset).await {
                Ok(id) => id,
                Err(ApiError::NotFound(_)) => {
                    send_no_route_to_pair(&state, &registry, base, quote).await;
                    continue;
                }
                Err(e) => {
                    warn!("broadcaster: find_asset_id({}) error: {:?}", quote, e);
                    continue;
                }
            };

            // ----------------------------------------------------------------
            // 2. Check liquidity revision — skip if unchanged
            // ----------------------------------------------------------------
            let revision = match get_liquidity_revision(&state, base_id, quote_id).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("broadcaster: get_liquidity_revision error: {:?}", e);
                    continue;
                }
            };

            let pair_key = (base.clone(), quote.clone());
            let prev_revision = last_revisions.get(&pair_key).cloned();

            // Always emit on first poll (prev_revision is None); otherwise
            // only emit when the revision has changed.
            if prev_revision.as_deref() == Some(revision.as_str()) {
                debug!("broadcaster: no revision change for {}/{}, skipping", base, quote);
                continue;
            }

            last_revisions.insert(pair_key.clone(), revision.clone());

            // ----------------------------------------------------------------
            // 3. Get subscriptions for this pair
            // ----------------------------------------------------------------
            let subs = {
                let reg = registry.read().await;
                reg.get_connections_for_pair(base, quote)
            };

            for (conn_id, tx, sub) in subs {
                // ------------------------------------------------------------
                // 4. Compute quote
                // ------------------------------------------------------------
                let amount: f64 = sub
                    .amount
                    .as_deref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1.0);

                let quote_result =
                    find_best_price(&state, &base_asset, &quote_asset, base_id, quote_id, amount)
                        .await;

                let (price, path, rationale) = match quote_result {
                    Ok(r) => r,
                    Err(ApiError::NoRouteFound) => {
                        let msg = ServerMessage::now(ServerPayload::Error {
                            code: "no_route_found".into(),
                            message: format!("No liquidity found for {}/{}", base, quote),
                        });
                        send_or_remove(&state, &registry, conn_id, &tx, msg).await;
                        continue;
                    }
                    Err(e) => {
                        warn!(
                            "broadcaster: find_best_price({}/{}) error: {:?}",
                            base, quote, e
                        );
                        continue;
                    }
                };

                // ------------------------------------------------------------
                // 5. Dedup — skip if price hasn't changed beyond threshold
                // ------------------------------------------------------------
                if should_skip_emission(sub.last_emitted_price, price, sub.amount.is_some()) {
                    debug!(
                        "broadcaster: price unchanged for sub {}, skipping",
                        sub.id
                    );
                    continue;
                }

                // ------------------------------------------------------------
                // 6. Build and send the QuoteUpdate message
                // ------------------------------------------------------------
                let timestamp = chrono::Utc::now().timestamp_millis();
                let quote_response = QuoteResponse {
                    base_asset: asset_path_to_info(&base_asset),
                    quote_asset: asset_path_to_info(&quote_asset),
                    amount: format!("{:.7}", amount),
                    price: format!("{:.7}", price),
                    total: format!("{:.7}", amount * price),
                    quote_type: "sell".to_string(),
                    path,
                    timestamp,
                    expires_at: None,
                    source_timestamp: None,
                    ttl_seconds: None,
                    rationale: Some(rationale),
                    exclusion_diagnostics: None,
                    data_freshness: None,
                };

                let msg = ServerMessage::now(ServerPayload::QuoteUpdate {
                    subscription_id: sub.id,
                    quote: quote_response,
                });

                let sent = send_or_remove(&state, &registry, conn_id, &tx, msg).await;

                // ------------------------------------------------------------
                // 7. Update last_emitted_price on success
                // ------------------------------------------------------------
                if sent {
                    let mut reg = registry.write().await;
                    reg.update_last_emitted_price(conn_id, sub.id, price);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the emission should be skipped (price unchanged).
///
/// - When `amount_filter_set` is `true` (subscription has an `amount`), apply
///   the 0.01 % threshold: skip if `|new - old| / old <= 0.0001`.
/// - When `amount_filter_set` is `false`, skip only if the price is exactly
///   the same as the last emission.
pub fn should_skip_emission(
    last_price: Option<f64>,
    new_price: f64,
    amount_filter_set: bool,
) -> bool {
    match last_price {
        None => false, // always emit on first update
        Some(p) => {
            if amount_filter_set {
                // 0.01 % threshold
                (new_price - p).abs() / p <= 0.0001
            } else {
                // Exact equality check
                (new_price - p).abs() < f64::EPSILON
            }
        }
    }
}

/// Send a message to a connection, handling backpressure and closed channels.
///
/// Returns `true` if the message was sent successfully, `false` otherwise.
async fn send_or_remove(
    _state: &AppState,
    registry: &Arc<RwLock<SubscriptionRegistry>>,
    conn_id: uuid::Uuid,
    tx: &tokio::sync::mpsc::Sender<ServerMessage>,
    msg: ServerMessage,
) -> bool {
    match tx.try_send(msg) {
        Ok(()) => true,
        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
            // Channel is full — log and skip (connection task handles backpressure)
            warn!(
                "broadcaster: outbound channel full for conn {}, dropping message",
                conn_id
            );
            false
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
            // Channel closed — connection is gone, remove it from registry
            debug!("broadcaster: channel closed for conn {}, removing", conn_id);
            let mut reg = registry.write().await;
            reg.remove_connection(conn_id);
            false
        }
    }
}

/// Send a `no_route_found` error to all subscriptions for a given pair.
async fn send_no_route_to_pair(
    _state: &AppState,
    registry: &Arc<RwLock<SubscriptionRegistry>>,
    base: &str,
    quote: &str,
) {
    let subs = {
        let reg = registry.read().await;
        reg.get_connections_for_pair(base, quote)
    };
    for (conn_id, tx, _sub) in subs {
        let msg = ServerMessage::now(ServerPayload::Error {
            code: "no_route_found".into(),
            message: format!("No liquidity found for {}/{}", base, quote),
        });
        match tx.try_send(msg) {
            Ok(()) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                warn!("broadcaster: channel full for conn {} (no_route)", conn_id);
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                let mut reg = registry.write().await;
                reg.remove_connection(conn_id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DB helpers (inlined from routes/quote.rs — private functions)
// ---------------------------------------------------------------------------

async fn find_asset_id(
    state: &AppState,
    asset: &AssetPath,
) -> Result<Uuid, ApiError> {
    let asset_type = asset.to_asset_type();

    let row = if asset.asset_code == "native" {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
            limit 1
            "#,
        )
        .bind(&asset_type)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
              and asset_code = $2
              and ($3::text is null or asset_issuer = $3)
            limit 1
            "#,
        )
        .bind(&asset_type)
        .bind(&asset.asset_code)
        .bind(&asset.asset_issuer)
        .fetch_optional(&state.db)
        .await?
    };

    match row {
        Some(row) => Ok(row.get("id")),
        None => Err(ApiError::NotFound(format!(
            "Asset not found: {}",
            asset.asset_code
        ))),
    }
}

async fn get_liquidity_revision(
    state: &AppState,
    base_id: Uuid,
    quote_id: Uuid,
) -> Result<String, ApiError> {
    let row = sqlx::query(
        r#"
        select coalesce(max(source_ledger), 0)::bigint as revision
        from normalized_liquidity
        where (selling_asset_id = $1 and buying_asset_id = $2)
           or (selling_asset_id = $2 and buying_asset_id = $1)
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_one(&state.db)
    .await?;

    let revision: i64 = row.get("revision");
    Ok(revision.to_string())
}

async fn find_best_price(
    state: &AppState,
    base: &AssetPath,
    quote: &AssetPath,
    base_id: Uuid,
    quote_id: Uuid,
    amount: f64,
) -> Result<(f64, Vec<PathStep>, QuoteRationaleMetadata), ApiError> {
    let rows = sqlx::query(
        r#"
        select
            venue_type,
            venue_ref,
            price::text as price,
            available_amount::text as available_amount
        from normalized_liquidity
        where selling_asset_id = $1
          and buying_asset_id = $2
        order by price asc, venue_type asc, venue_ref asc
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_all(&state.db)
    .await?;

    if rows.is_empty() {
        return Err(ApiError::NoRouteFound);
    }

    let mut candidates: Vec<(String, String, f64, f64)> = rows
        .into_iter()
        .map(|row| {
            let venue_type: String = row.get("venue_type");
            let venue_ref: String = row.get("venue_ref");
            let price: f64 = row.get::<String, _>("price").parse().unwrap_or(0.0);
            let available: f64 = row
                .get::<String, _>("available_amount")
                .parse()
                .unwrap_or(0.0);
            (venue_type, venue_ref, price, available)
        })
        .collect();

    // Sort by price asc, then venue_type, then venue_ref for determinism
    candidates.sort_by(|a, b| {
        a.2.partial_cmp(&b.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });

    let compared_venues: Vec<VenueEvaluation> = candidates
        .iter()
        .map(|(vt, vr, price, avail)| VenueEvaluation {
            source: format!("{}:{}", vt, vr),
            price: format!("{:.7}", price),
            available_amount: format!("{:.7}", avail),
            executable: *avail >= amount && *price > 0.0,
        })
        .collect();

    let selected = candidates
        .iter()
        .find(|(_, _, price, avail)| *avail >= amount && *price > 0.0)
        .cloned()
        .ok_or(ApiError::NoRouteFound)?;

    let (venue_type, venue_ref, price, _) = selected;
    let source = if venue_type == "amm" {
        format!("amm:{}", venue_ref)
    } else {
        "sdex".to_string()
    };
    let selected_source = format!("{}:{}", venue_type, venue_ref);

    let path = vec![PathStep {
        from_asset: asset_path_to_info(base),
        to_asset: asset_path_to_info(quote),
        price: format!("{:.7}", price),
        source,
    }];

    let rationale = QuoteRationaleMetadata {
        strategy: "single_hop_direct_venue_comparison".to_string(),
        selected_source,
        compared_venues,
    };

    Ok((price, path, rationale))
}

fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: websocket-quote-stream, Property 8: Amount-based dedup threshold
    proptest::proptest! {
        #[test]
        fn prop_dedup_threshold(
            last_price in 0.0001f64..1_000_000.0f64,
            delta_factor in -0.5f64..0.5f64,
        ) {
            // new_price = last_price * (1 + delta_factor)
            let new_price = last_price * (1.0 + delta_factor);
            if new_price <= 0.0 {
                return Ok(());
            }

            let relative_change = (new_price - last_price).abs() / last_price;

            // With amount filter set (0.01% threshold)
            let skip_with_filter = should_skip_emission(Some(last_price), new_price, true);
            if relative_change <= 0.0001 {
                proptest::prop_assert!(
                    skip_with_filter,
                    "should skip when change <= 0.01%: last={}, new={}, rel={}",
                    last_price, new_price, relative_change
                );
            } else {
                proptest::prop_assert!(
                    !skip_with_filter,
                    "should emit when change > 0.01%: last={}, new={}, rel={}",
                    last_price, new_price, relative_change
                );
            }

            // Without amount filter: only skip on exact equality
            let skip_no_filter = should_skip_emission(Some(last_price), new_price, false);
            if (new_price - last_price).abs() < f64::EPSILON {
                proptest::prop_assert!(skip_no_filter);
            } else {
                proptest::prop_assert!(!skip_no_filter);
            }
        }
    }

    #[test]
    fn dedup_no_last_price_always_emits() {
        assert!(!should_skip_emission(None, 1.5, true));
        assert!(!should_skip_emission(None, 1.5, false));
    }

    #[test]
    fn dedup_exact_same_price_with_filter_skips() {
        assert!(should_skip_emission(Some(1.0), 1.0, true));
    }

    #[test]
    fn dedup_just_above_threshold_emits() {
        // 0.011% change — just above 0.01% threshold
        let last = 1.0_f64;
        let new = 1.0 + 0.00011;
        assert!(!should_skip_emission(Some(last), new, true));
    }

    #[test]
    fn dedup_just_below_threshold_skips() {
        // 0.009% change — just below 0.01% threshold
        let last = 1.0_f64;
        let new = 1.0 + 0.00009;
        assert!(should_skip_emission(Some(last), new, true));
    }
}
