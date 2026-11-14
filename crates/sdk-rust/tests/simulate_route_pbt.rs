//! Property-based tests for the `sdk-rust-simulate-route` feature.
//!
//! Tests the serde round-trip and wire-shape invariants for the new
//! `SimulateRouteRequest` and `SimulateRouteResponse` types without
//! requiring a live API or database.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk --test simulate_route_pbt

use proptest::prelude::*;
use stellarroute_sdk::{
    AssetInfo, DryRunHop, PathStep, SimulateQuoteResult, SimulateRouteRequest,
    SimulateRouteResponse, SlippageOverride, SwapHopDto, SwapPathDto,
};

// ── Proptest strategies ───────────────────────────────────────────────────────

/// Strategy for arbitrary `AssetInfo` values.
fn arb_asset_info() -> impl Strategy<Value = AssetInfo> {
    let native = Just(AssetInfo {
        asset_type: "native".to_string(),
        asset_code: None,
        asset_issuer: None,
    });
    let issued = ("[A-Z]{1,12}", option::of("[A-Z0-9]{4,56}")).prop_map(|(code, issuer)| {
        let asset_type = if code.len() <= 4 {
            "credit_alphanum4"
        } else {
            "credit_alphanum12"
        };
        AssetInfo {
            asset_type: asset_type.to_string(),
            asset_code: Some(code),
            asset_issuer: issuer,
        }
    });
    prop_oneof![native, issued]
}

/// Strategy for arbitrary `PathStep` values.
fn arb_path_step() -> impl Strategy<Value = PathStep> {
    (
        arb_asset_info(),
        arb_asset_info(),
        "[0-9.]{3,12}",
        "[a-z]{3,6}",
    )
        .prop_map(|(from_asset, to_asset, price, source)| PathStep {
            from_asset,
            to_asset,
            price,
            source,
        })
}

/// Strategy for arbitrary `DryRunHop` values.
fn arb_dry_run_hop() -> impl Strategy<Value = DryRunHop> {
    (
        "[a-z0-9:A-Z]{4,20}",
        "[a-z0-9:A-Z]{4,20}",
        "[a-z_]{3,10}",
        option::of(any::<u32>()),
        option::of("[0-9.]{3,12}"),
        option::of("[a-z_]{3,10}"),
    )
        .prop_map(
            |(from_asset, to_asset, source, fee_bps, price, venue_ref)| DryRunHop {
                from_asset,
                to_asset,
                source,
                fee_bps,
                price,
                venue_ref,
            },
        )
}

/// Strategy for arbitrary `SwapHopDto` values.
fn arb_swap_hop_dto() -> impl Strategy<Value = SwapHopDto> {
    (
        "[a-z0-9:A-Z]{4,20}",
        "[a-z0-9:A-Z]{4,20}",
        prop_oneof![Just("sdex".to_string()), Just("amm".to_string())],
        "[a-z0-9:]{3,15}",
        any::<f64>().prop_filter("finite", |f| f.is_finite() && *f >= 0.0),
        any::<u32>(),
    )
        .prop_map(
            |(source_asset, destination_asset, venue_type, venue_ref, price, fee_bps)| SwapHopDto {
                source_asset,
                destination_asset,
                venue_type,
                venue_ref,
                price,
                fee_bps,
            },
        )
}

/// Strategy for arbitrary `SwapPathDto` values.
fn arb_swap_path_dto() -> impl Strategy<Value = SwapPathDto> {
    (
        proptest::collection::vec(arb_swap_hop_dto(), 0..=5),
        any::<i64>(),
    )
        .prop_map(|(hops, estimated_output)| SwapPathDto {
            hops,
            estimated_output,
        })
}

/// Strategy for arbitrary `SimulateQuoteResult` values.
fn arb_simulate_quote_result() -> impl Strategy<Value = SimulateQuoteResult> {
    (
        arb_asset_info(),
        arb_asset_info(),
        "[0-9.]{3,12}",
        "[0-9.]{3,12}",
        "[0-9.]{3,12}",
        any::<bool>(),
        proptest::collection::vec(arb_path_step(), 0..=3),
        any::<i64>(),
    )
        .prop_map(
            |(base_asset, quote_asset, amount, price, total, degraded, path, timestamp)| {
                SimulateQuoteResult {
                    base_asset,
                    quote_asset,
                    amount,
                    price,
                    total,
                    quote_type: "sell".to_string(),
                    degraded,
                    path,
                    timestamp,
                    expires_at: None,
                    source_timestamp: None,
                    ttl_seconds: None,
                    rationale: None,
                    exclusion_diagnostics: None,
                    data_freshness: None,
                    midpoint: None,
                    spread_bps: None,
                    price_impact: None,
                }
            },
        )
}

/// Strategy for arbitrary `SimulateRouteResponse` values.
fn arb_simulate_route_response() -> impl Strategy<Value = SimulateRouteResponse> {
    (arb_simulate_quote_result(), arb_swap_path_dto()).prop_map(|(quote, swap_path)| {
        SimulateRouteResponse {
            quote,
            exclusion_diagnostics: None,
            swap_path,
        }
    })
}

// ── Property 1: SimulateRouteResponse round-trip serialization ────────────────
//
// Feature: sdk-rust-simulate-route, Property 1: SimulateRouteResponse round-trip serialization
// Validates: Requirements 1.1, 1.2, 1.4, 1.5, 1.6, 1.9, 4.7

proptest! {
    #[test]
    fn simulate_route_response_serde_roundtrip(
        response in arb_simulate_route_response()
    ) {
        let json = serde_json::to_string(&response)
            .expect("serialization should succeed");
        let decoded: SimulateRouteResponse = serde_json::from_str(&json)
            .expect("deserialization of own output should succeed");

        // Structural equality on key fields.
        prop_assert_eq!(&decoded.quote.price, &response.quote.price);
        prop_assert_eq!(&decoded.quote.total, &response.quote.total);
        prop_assert_eq!(&decoded.quote.amount, &response.quote.amount);
        prop_assert_eq!(decoded.quote.degraded, response.quote.degraded);
        prop_assert_eq!(decoded.quote.timestamp, response.quote.timestamp);
        prop_assert_eq!(decoded.quote.path.len(), response.quote.path.len());
        prop_assert_eq!(decoded.swap_path.hops.len(), response.swap_path.hops.len());
        prop_assert_eq!(decoded.swap_path.estimated_output, response.swap_path.estimated_output);
        prop_assert!(decoded.exclusion_diagnostics.is_none());
    }
}

// ── Property 2: Wire-shape invariant for SimulateRouteRequest ─────────────────
//
// Feature: sdk-rust-simulate-route, Property 2: Wire-shape invariant for SimulateRouteRequest
// Validates: Requirements 1.7, 2.2, 4.8

proptest! {
    #[test]
    fn simulate_route_request_wire_shape(
        hops in proptest::collection::vec(arb_dry_run_hop(), 1..=5),
        slippage_bps in option::of(any::<u32>()),
        overrides in proptest::collection::vec(
            ("[a-z]{3,8}", any::<u32>()).prop_map(|(vr, bps)| SlippageOverride {
                venue_ref: vr,
                slippage_bps: bps,
            }),
            0..=3,
        ),
    ) {
        let n_hops = hops.len();
        let req = SimulateRouteRequest {
            hops,
            amount: "100.0".to_string(),
            slippage_bps,
            slippage_bps_overrides: overrides,
        };

        let json: serde_json::Value = serde_json::to_value(&req)
            .expect("SimulateRouteRequest should serialize");

        // Top-level "route" key must exist and contain "hops".
        prop_assert!(
            json["route"]["hops"].is_array(),
            "expected json[\"route\"][\"hops\"] to be an array, got: {json}"
        );

        // Hops count must be preserved.
        let serialized_len = json["route"]["hops"].as_array().unwrap().len();
        prop_assert_eq!(
            serialized_len,
            n_hops,
            "hop count mismatch: expected {n_hops}, got {serialized_len}"
        );

        // "amount" must be at the top level, not nested inside "route".
        prop_assert!(
            json["amount"].is_string(),
            "expected top-level \"amount\" key, got: {json}"
        );

        // "hops" must NOT appear at the top level (they belong under route).
        prop_assert!(
            json.get("hops").is_none(),
            "\"hops\" should not appear at the top level: {json}"
        );
    }
}
