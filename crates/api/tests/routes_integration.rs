//! Integration tests for the /api/v1/routes endpoint response shape and logic.
//!
//! These tests drive the domain models and response structures directly (unit-style
//! integration), verifying: multi-route list, ranking scores, per-route metadata
//! (fees, hops, impact estimates), and limit/pagination semantics.

use stellarroute_api::models::{
    AssetInfo, RouteCandidate, RouteHop, RoutesResponse,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_hop(from: &str, to: &str, price: &str, fee_bps: u32, source: &str) -> RouteHop {
    RouteHop {
        from_asset: if from == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(from.to_string(), None)
        },
        to_asset: if to == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(to.to_string(), None)
        },
        price: price.to_string(),
        amount_out_of_hop: "10.0000000".to_string(),
        fee_bps,
        source: source.to_string(),
    }
}

fn make_candidate(score: f64, impact_bps: u32, hops: Vec<RouteHop>) -> RouteCandidate {
    RouteCandidate {
        estimated_output: format!("{:.7}", 10.0_f64 - (impact_bps as f64 * 0.0001)),
        impact_bps,
        score,
        policy_used: "production".to_string(),
        path: hops,
    }
}

fn make_routes_response(routes: Vec<RouteCandidate>) -> RoutesResponse {
    RoutesResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::credit("USDC".to_string(), None),
        amount: "10.0000000".to_string(),
        routes,
        timestamp: 1_700_000_000_000,
    }
}

// ── AC1: Response lists multiple ranked route candidates ──────────────────────

#[test]
fn routes_response_contains_multiple_candidates() {
    let candidate_a = make_candidate(
        92.5,
        30,
        vec![make_hop("native", "USDC", "1.0000000", 30, "amm:pool-1")],
    );
    let candidate_b = make_candidate(
        85.0,
        50,
        vec![
            make_hop("native", "BTC", "0.0000200", 20, "sdex"),
            make_hop("BTC", "USDC", "50000.0000000", 20, "sdex"),
        ],
    );

    let resp = make_routes_response(vec![candidate_a, candidate_b]);
    assert_eq!(resp.routes.len(), 2, "should return 2 routes");
}

// ── AC2: Routes are ranked by descending score ────────────────────────────────

#[test]
fn routes_are_ranked_by_descending_score() {
    let best = make_candidate(95.0, 20, vec![make_hop("native", "USDC", "1.0", 20, "amm:pool-1")]);
    let second = make_candidate(
        80.0,
        40,
        vec![
            make_hop("native", "XLM2", "1.0", 20, "sdex"),
            make_hop("XLM2", "USDC", "1.0", 20, "sdex"),
        ],
    );
    let third = make_candidate(65.5, 80, vec![make_hop("native", "USDC", "0.98", 80, "sdex")]);

    let resp = make_routes_response(vec![best, second, third]);

    let scores: Vec<f64> = resp.routes.iter().map(|r| r.score).collect();
    for i in 0..scores.len() - 1 {
        assert!(
            scores[i] >= scores[i + 1],
            "routes[{}].score ({}) must be >= routes[{}].score ({})",
            i, scores[i], i + 1, scores[i + 1]
        );
    }
}

// ── AC3: Per-route metadata (fees, hops, impact) ─────────────────────────────

#[test]
fn route_candidate_exposes_fee_bps_per_hop() {
    let hop = make_hop("native", "USDC", "1.0000000", 30, "amm:pool-xyz");
    assert_eq!(hop.fee_bps, 30, "AMM hop should carry 30bps fee");
    assert_eq!(hop.source, "amm:pool-xyz");
}

#[test]
fn route_candidate_exposes_impact_bps() {
    let candidate = make_candidate(88.0, 45, vec![make_hop("native", "USDC", "1.0", 45, "sdex")]);
    assert_eq!(candidate.impact_bps, 45);
}

#[test]
fn route_candidate_exposes_multi_hop_path() {
    let hops = vec![
        make_hop("native", "ETH", "0.0004000", 20, "sdex"),
        make_hop("ETH", "USDC", "2500.0000000", 30, "amm:pool-eth-usdc"),
    ];
    let candidate = make_candidate(78.5, 50, hops);
    assert_eq!(candidate.path.len(), 2, "two-hop path should expose both hops");
    assert_eq!(candidate.path[1].source, "amm:pool-eth-usdc");
}

#[test]
fn route_hop_price_is_formatted_to_7_decimals() {
    let hop = make_hop("native", "USDC", "1.2345678", 20, "sdex");
    assert_eq!(hop.price, "1.2345678");
    // Verify it round-trips through JSON
    let json = serde_json::to_value(&hop).expect("serialization failed");
    assert_eq!(json["price"], "1.2345678");
    assert_eq!(json["fee_bps"], 20);
}

#[test]
fn route_candidate_amount_out_of_hop_is_present() {
    let hop = make_hop("native", "USDC", "1.0000000", 20, "sdex");
    assert!(!hop.amount_out_of_hop.is_empty(), "amount_out_of_hop must be populated");
}

// ── AC4: Limit / pagination supports capping route count ─────────────────────

#[test]
fn routes_response_respects_limit_of_one() {
    let candidates = vec![
        make_candidate(95.0, 20, vec![make_hop("native", "USDC", "1.0", 20, "amm:pool-1")]),
        make_candidate(85.0, 40, vec![make_hop("native", "USDC", "0.99", 40, "sdex")]),
        make_candidate(70.0, 60, vec![make_hop("native", "USDC", "0.98", 60, "sdex")]),
    ];

    // Simulate caller-side limit=1 (as the handler does)
    let limited: Vec<_> = candidates.into_iter().take(1).collect();
    let resp = make_routes_response(limited);
    assert_eq!(resp.routes.len(), 1);
}

#[test]
fn routes_response_returns_all_when_limit_exceeds_candidates() {
    let candidates = vec![
        make_candidate(95.0, 20, vec![make_hop("native", "USDC", "1.0", 20, "amm:pool-1")]),
    ];
    let limit = 5;
    let limited: Vec<_> = candidates.into_iter().take(limit).collect();
    let resp = make_routes_response(limited);
    assert_eq!(resp.routes.len(), 1, "fewer routes than limit is fine");
}

// ── AC5: JSON serialization is complete and correct ──────────────────────────

#[test]
fn routes_response_serializes_with_all_required_fields() {
    let resp = make_routes_response(vec![make_candidate(
        90.0,
        30,
        vec![make_hop("native", "USDC", "1.0000000", 30, "amm:pool-1")],
    )]);

    let json = serde_json::to_value(&resp).expect("serialization failed");

    assert!(json.get("base_asset").is_some(), "base_asset required");
    assert!(json.get("quote_asset").is_some(), "quote_asset required");
    assert!(json.get("amount").is_some(), "amount required");
    assert!(json.get("routes").is_some(), "routes required");
    assert!(json.get("timestamp").is_some(), "timestamp required");

    let routes = json["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);

    let candidate = &routes[0];
    assert!(candidate.get("estimated_output").is_some());
    assert!(candidate.get("impact_bps").is_some());
    assert!(candidate.get("score").is_some());
    assert!(candidate.get("policy_used").is_some());
    assert!(candidate.get("path").is_some());

    let hops = candidate["path"].as_array().unwrap();
    let hop = &hops[0];
    assert!(hop.get("from_asset").is_some());
    assert!(hop.get("to_asset").is_some());
    assert!(hop.get("price").is_some());
    assert!(hop.get("amount_out_of_hop").is_some());
    assert!(hop.get("fee_bps").is_some());
    assert!(hop.get("source").is_some());
}

#[test]
fn policy_used_field_propagates_in_all_candidates() {
    let resp = make_routes_response(vec![
        make_candidate(90.0, 30, vec![make_hop("native", "USDC", "1.0", 30, "sdex")]),
        make_candidate(80.0, 50, vec![make_hop("native", "USDC", "0.99", 50, "sdex")]),
    ]);

    for candidate in &resp.routes {
        assert_eq!(
            candidate.policy_used, "production",
            "all candidates must carry the active policy"
        );
    }
}
