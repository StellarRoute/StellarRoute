use criterion::{black_box, criterion_group, criterion_main, Criterion};
use stellarroute_api::models::{
    AssetInfo, DataFreshness, ExcludedVenueInfo, ExclusionDiagnostics, ExclusionReason, PathStep,
    PreparedQuoteResponse, QuoteRationaleMetadata, QuoteResponse, VenueEvaluation,
};

fn sample_quote_response() -> QuoteResponse {
    let native = AssetInfo::native();
    let usdc = AssetInfo::credit(
        "USDC".to_string(),
        Some("GBBD67SIWK6V6I7SGPW76BGSYDBBCOOT6YF7KOCUT5NJSWJRXFNY6X3K".to_string()),
    );
    let eurc = AssetInfo::credit(
        "EURC".to_string(),
        Some("GCPAWY6H77Q5Z6K7R2YB4C3MUKQ5M2R6VME6H7AB7D5VZ6X7FJ4QK6T4".to_string()),
    );

    QuoteResponse {
        base_asset: native.clone(),
        quote_asset: usdc.clone(),
        amount: "25000.0000000".to_string(),
        price: "0.9987421".to_string(),
        total: "24968.5525000".to_string(),
        quote_type: "sell".to_string(),
        path: vec![
            PathStep {
                from_asset: native.clone(),
                to_asset: eurc.clone(),
                price: "0.9991000".to_string(),
                source: "sdex".to_string(),
            },
            PathStep {
                from_asset: eurc,
                to_asset: usdc.clone(),
                price: "0.9996420".to_string(),
                source: "amm:pool-eurc-usdc".to_string(),
            },
        ],
        timestamp: 1_712_345_678_901,
        expires_at: Some(1_712_345_680_901),
        source_timestamp: Some(1_712_345_678_500),
        ttl_seconds: Some(2),
        rationale: Some(QuoteRationaleMetadata {
            strategy: "best_executable_direct_venue".to_string(),
            selected_source: "amm:pool-eurc-usdc".to_string(),
            compared_venues: vec![
                VenueEvaluation {
                    source: "sdex".to_string(),
                    price: "0.9984000".to_string(),
                    available_amount: "24000.0000000".to_string(),
                    executable: false,
                },
                VenueEvaluation {
                    source: "amm:pool-eurc-usdc".to_string(),
                    price: "0.9987421".to_string(),
                    available_amount: "30000.0000000".to_string(),
                    executable: true,
                },
            ],
        }),
        price_impact: Some("0.18".to_string()),
        exclusion_diagnostics: Some(ExclusionDiagnostics {
            excluded_venues: vec![
                ExcludedVenueInfo {
                    venue_ref: "sdex:offer-1".to_string(),
                    reason: ExclusionReason::PolicyThreshold { threshold: 0.72 },
                },
                ExcludedVenueInfo {
                    venue_ref: "amm:pool-legacy".to_string(),
                    reason: ExclusionReason::StaleData,
                },
            ],
        }),
        data_freshness: Some(DataFreshness {
            fresh_count: 5,
            stale_count: 1,
            max_staleness_secs: 27,
        }),
    }
}

fn bench_quote_serialization(c: &mut Criterion) {
    let quote = sample_quote_response();
    c.bench_function("quote_response_serialize_each_time", |b| {
        b.iter(|| serde_json::to_vec(black_box(&quote)).expect("serialize quote response"))
    });
}

fn bench_prepared_quote_response(c: &mut Criterion) {
    let prepared =
        PreparedQuoteResponse::from_quote(sample_quote_response()).expect("prepare quote");
    c.bench_function("quote_response_cached_json_reuse", |b| {
        b.iter(|| black_box(prepared.json_bytes().clone()))
    });
}

criterion_group!(
    benches,
    bench_quote_serialization,
    bench_prepared_quote_response
);
criterion_main!(benches);
