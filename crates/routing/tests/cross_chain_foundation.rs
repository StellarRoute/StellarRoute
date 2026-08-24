//! Foundation tests: non-executable bridges, SLIP-44 natives, provider kill-switch,
//! shared canonicalize fixtures, bridge meta consistency, additive serde.

use stellarroute_routing::chain_asset::{
    canonicalize_asset_id, canonicalize_for_v1_cache, AssetReference, ChainAsset, ChainId,
};
use stellarroute_routing::cross_chain::{BridgeEdgeMeta, ProviderPolicy};
use stellarroute_routing::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig};
use stellarroute_routing::policy::RoutingPolicy;
use stellarroute_routing::risk::{ExclusionReason, RiskLimitConfig, RiskValidator};

const VALID_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const USDC_ETH: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

fn stellar_xlm() -> String {
    ChainAsset::stellar_native("pubnet").to_canonical()
}

fn stellar_usdc() -> String {
    ChainAsset::stellar_credit("pubnet", "USDC", VALID_ISSUER)
        .unwrap()
        .to_canonical()
}

fn eth_usdc() -> String {
    ChainAsset::new(
        ChainId::ethereum_mainnet(),
        AssetReference::Erc20 {
            address: USDC_ETH.into(),
        },
    )
    .unwrap()
    .to_canonical()
}

fn eth_native() -> String {
    ChainAsset::new(ChainId::ethereum_mainnet(), AssetReference::Native)
        .unwrap()
        .to_canonical()
}

fn dex_edge(from: &str, to: &str, venue_ref: &str, provider: Option<&str>) -> LiquidityEdge {
    LiquidityEdge {
        from: from.to_string(),
        to: to.to_string(),
        venue_type: "amm".to_string(),
        venue_ref: venue_ref.to_string(),
        liquidity: 10_000_000_000,
        price: 1.0,
        fee_bps: 30,
        provider: provider.map(|p| p.to_string()),
        bridge: None,
    }
}

fn bridge_edge(
    from: &str,
    to: &str,
    provider: &str,
    src: &ChainId,
    dst: &ChainId,
) -> LiquidityEdge {
    LiquidityEdge {
        from: from.to_string(),
        to: to.to_string(),
        venue_type: "bridge".to_string(),
        venue_ref: format!("{provider}:lane"),
        liquidity: 50_000_000_000,
        price: 1.0,
        fee_bps: 10,
        provider: Some(provider.to_string()),
        bridge: Some(BridgeEdgeMeta::bridge(provider, src, dst)),
    }
}

#[test]
fn default_policy_hard_disables_bridge_edges() {
    let xlm = stellar_xlm();
    let s_usdc = stellar_usdc();
    let e_usdc = eth_usdc();
    let eth = eth_native();

    let edges = vec![
        dex_edge(&xlm, &s_usdc, "stellar-pool-1", Some("stellar-amm")),
        bridge_edge(
            &s_usdc,
            &e_usdc,
            "example-bridge",
            &ChainId::stellar_pubnet(),
            &ChainId::ethereum_mainnet(),
        ),
        dex_edge(&e_usdc, &eth, "uniswap-pool-1", Some("uniswap")),
    ];

    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let policy = RoutingPolicy::default().with_max_hops(4);
    assert!(!policy.allow_bridge_edges);

    let err = finder
        .find_paths(&xlm, &eth, &edges, 1_000_000, &policy)
        .expect_err("bridges must be non-executable under default policy");
    assert!(matches!(
        err,
        stellarroute_routing::error::RoutingError::NoRoute(_, _)
    ));
}

#[test]
fn bridge_meta_on_amm_venue_still_non_executable() {
    let xlm = stellar_xlm();
    let e_usdc = eth_usdc();
    let mut edge = dex_edge(&xlm, &e_usdc, "sneaky", Some("example-bridge"));
    edge.bridge = Some(BridgeEdgeMeta::bridge(
        "example-bridge",
        &ChainId::stellar_pubnet(),
        &ChainId::ethereum_mainnet(),
    ));

    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let err = finder
        .find_paths(&xlm, &e_usdc, &[edge], 1_000_000, &RoutingPolicy::default())
        .expect_err("bridge metadata must disable the edge");
    assert!(matches!(
        err,
        stellarroute_routing::error::RoutingError::NoRoute(_, _)
    ));
}

#[test]
fn opt_in_bridge_still_requires_consistent_meta() {
    let s_usdc = stellar_usdc();
    let e_usdc = eth_usdc();
    let bad = bridge_edge(
        &s_usdc,
        &e_usdc,
        "example-bridge",
        &ChainId::ethereum_mainnet(), // swapped — inconsistent
        &ChainId::stellar_pubnet(),
    );

    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let policy = RoutingPolicy::default().with_allow_bridge_edges(true);
    let err = finder
        .find_paths(&s_usdc, &e_usdc, &[bad], 1_000_000, &policy)
        .expect_err("inconsistent bridge meta must not route");
    assert!(matches!(
        err,
        stellarroute_routing::error::RoutingError::NoRoute(_, _)
    ));
}

#[test]
fn provider_kill_switch_excludes_dex_provider_edges() {
    let xlm = stellar_xlm();
    let s_usdc = stellar_usdc();

    let edges = vec![
        dex_edge(&xlm, &s_usdc, "pool-a", Some("bad-dex")),
        dex_edge(&xlm, &s_usdc, "pool-b", Some("good-dex")),
    ];

    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let policy = RoutingPolicy::default()
        .with_provider_policy(ProviderPolicy::default().with_kill_switch("bad-dex", true));

    let paths = finder
        .find_paths(&xlm, &s_usdc, &edges, 1_000_000, &policy)
        .expect("good-dex route");
    assert!(!paths.is_empty());
    for path in &paths {
        for hop in &path.hops {
            assert_ne!(hop.provider.as_deref(), Some("bad-dex"));
        }
    }
}

#[test]
fn risk_validator_enforces_provider_on_route_hops() {
    let config = RiskLimitConfig::default().with_provider_kill_switch("bad-bridge", true);
    let validator = RiskValidator::new(config);

    let hops = vec![
        (
            stellar_xlm(),
            1_000_i128,
            10_u32,
            10_000_000_i128,
            Some("ok-dex".into()),
        ),
        (
            stellar_usdc(),
            1_000_i128,
            10_u32,
            10_000_000_i128,
            Some("bad-bridge".into()),
        ),
    ];
    let err = validator.validate_route_hops(&hops).unwrap_err();
    assert!(err
        .iter()
        .any(|e| e.reason == ExclusionReason::ProviderKillSwitch));
}

#[test]
fn shared_canonicalize_fixture_vectors() {
    let raw = include_str!("fixtures/chain_asset_vectors.json");
    let vectors: serde_json::Value = serde_json::from_str(raw).unwrap();

    for item in vectors["canonical_round_trips"].as_array().unwrap() {
        let input = item["input"].as_str().unwrap();
        let expected = item["canonical"].as_str().unwrap();
        assert_eq!(
            canonicalize_asset_id(input).unwrap(),
            expected,
            "round-trip failed for {input}"
        );
    }
    for input in vectors["must_reject"].as_array().unwrap() {
        let input = input.as_str().unwrap();
        assert!(
            canonicalize_asset_id(input).is_err(),
            "expected reject for {input}"
        );
    }
    for item in vectors["v1_cache_legacy"].as_array().unwrap() {
        let input = item["input"].as_str().unwrap();
        let expected = item["canonical"].as_str().unwrap();
        assert_eq!(
            canonicalize_for_v1_cache(input).unwrap(),
            expected,
            "v1 cache mismatch for {input}"
        );
    }
}

#[test]
fn symbol_collision_stellar_vs_ethereum_usdc() {
    let a = canonicalize_asset_id(&format!("USDC:{VALID_ISSUER}")).unwrap();
    let b = canonicalize_asset_id(&format!("eip155:1/erc20:{USDC_ETH}")).unwrap();
    assert_ne!(a, b);
    assert!(a.starts_with("stellar:pubnet/stellar:USDC:"));
    assert!(b.starts_with("eip155:1/erc20:0xa0b8"));
}

#[test]
fn additive_serde_for_liquidity_edge() {
    let edge = bridge_edge(
        &stellar_usdc(),
        &eth_usdc(),
        "example-bridge",
        &ChainId::stellar_pubnet(),
        &ChainId::ethereum_mainnet(),
    );
    let json = serde_json::to_string(&edge).unwrap();
    let parsed: LiquidityEdge = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.provider.as_deref(), Some("example-bridge"));
    assert!(parsed
        .bridge
        .as_ref()
        .unwrap()
        .validate_against_endpoints(&parsed.from, &parsed.to)
        .is_ok());

    let legacy = r#"{
        "from":"native","to":"USDC","venue_type":"sdex",
        "venue_ref":"1","liquidity":100,"price":1.0,"fee_bps":30
    }"#;
    let legacy_edge: LiquidityEdge = serde_json::from_str(legacy).unwrap();
    assert!(legacy_edge.provider.is_none());
    assert!(legacy_edge.bridge.is_none());
}

#[test]
fn cycle_prevention_same_chain_still_works() {
    let xlm = stellar_xlm();
    let s_usdc = stellar_usdc();
    let edges = vec![
        dex_edge(&xlm, &s_usdc, "s1", None),
        dex_edge(&s_usdc, &xlm, "s1-rev", None),
    ];
    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let paths = finder
        .find_paths(&xlm, &s_usdc, &edges, 1_000_000, &RoutingPolicy::default())
        .unwrap();
    for path in &paths {
        let mut nodes = std::collections::HashSet::new();
        nodes.insert(path.hops[0].source_asset.clone());
        for hop in &path.hops {
            assert!(nodes.insert(hop.destination_asset.clone()));
        }
    }
}
