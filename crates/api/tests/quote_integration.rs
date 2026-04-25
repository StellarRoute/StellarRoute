//! Integration-style tests for quote response shape and deterministic rationale metadata.

use stellarroute_api::models::{
    AssetInfo, PathStep, QuoteRationaleMetadata, QuoteResponse, VenueEvaluation,
};

#[test]
fn quote_response_includes_rationale_metadata() {
    let response = QuoteResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::credit("USDC".to_string(), None),
        amount: "50.0000000".to_string(),
        price: "1.0000000".to_string(),
        total: "50.0000000".to_string(),
        quote_type: "sell".to_string(),
        path: vec![PathStep {
            from_asset: AssetInfo::native(),
            to_asset: AssetInfo::credit("USDC".to_string(), None),
            price: "1.0000000".to_string(),
            source: "sdex".to_string(),
        }],
        timestamp: 1_700_000_000,
        expires_at: Some(1_700_000_030_000),
        source_timestamp: None,
        ttl_seconds: Some(30),
        rationale: Some(QuoteRationaleMetadata {
            strategy: "single_hop_direct_venue_comparison".to_string(),
            selected_source: "sdex:offer-1".to_string(),
            compared_venues: vec![
                VenueEvaluation {
                    source: "amm:pool-1".to_string(),
                    price: "1.0000000".to_string(),
                    available_amount: "100.0000000".to_string(),
                    executable: true,
                },
                VenueEvaluation {
                    source: "sdex:offer-1".to_string(),
                    price: "0.9990000".to_string(),
                    available_amount: "75.0000000".to_string(),
                    executable: true,
                },
            ],
        }),
        price_impact: None,
        exclusion_diagnostics: None,
        data_freshness: None,
    };

    let json = serde_json::to_value(&response).expect("serialization failed");

    assert_eq!(
        json["rationale"]["strategy"],
        "single_hop_direct_venue_comparison"
    );
    assert_eq!(json["rationale"]["selected_source"], "sdex:offer-1");
    assert_eq!(
        json["rationale"]["compared_venues"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn rationale_venue_order_is_deterministic_in_json() {
    let rationale = QuoteRationaleMetadata {
        strategy: "single_hop_direct_venue_comparison".to_string(),
        selected_source: "amm:pool-a".to_string(),
        compared_venues: vec![
            VenueEvaluation {
                source: "amm:pool-a".to_string(),
                price: "1.0000000".to_string(),
                available_amount: "25.0000000".to_string(),
                executable: true,
            },
            VenueEvaluation {
                source: "sdex:offer-a".to_string(),
                price: "1.0000000".to_string(),
                available_amount: "25.0000000".to_string(),
                executable: true,
            },
        ],
    };

    let json = serde_json::to_string(&rationale).expect("serialization failed");
    let amm_pos = json.find("amm:pool-a").expect("missing amm source");
    let sdex_pos = json.find("sdex:offer-a").expect("missing sdex source");

    assert!(
        amm_pos < sdex_pos,
        "venue order should remain deterministic"
    );
}

#[test]
fn exclusion_diagnostics_venue_order_is_deterministic_in_json() {
    use stellarroute_api::models::{ExcludedVenueInfo, ExclusionDiagnostics, ExclusionReason};

    let diagnostics = ExclusionDiagnostics {
        excluded_venues: vec![
            ExcludedVenueInfo {
                venue_ref: "venue:bad_amm".to_string(),
                reason: ExclusionReason::StaleData,
            },
            ExcludedVenueInfo {
                venue_ref: "venue:bad_sdex".to_string(),
                reason: ExclusionReason::PolicyThreshold { threshold: 0.1 },
            },
        ],
    };

    let json = serde_json::to_string(&diagnostics).expect("serialization failed");
    let amm_pos = json.find("venue:bad_amm").expect("missing amm source");
    let sdex_pos = json.find("venue:bad_sdex").expect("missing sdex source");

    assert!(
        amm_pos < sdex_pos,
        "exclusion order should remain deterministic"
    );

    // ensure sensitive internals are not leaked (score and signals)
    assert!(!json.contains("score"));
    assert!(!json.contains("signals"));
}
