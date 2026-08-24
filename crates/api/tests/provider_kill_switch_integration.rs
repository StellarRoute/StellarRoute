//! Provider kill-switch plumbing: admin/Redis state → routing policy → pathfinder.
//!
//! Bridges remain non-executable; this proves DEX provider metadata is enforced.

use std::collections::HashMap;
use stellarroute_api::kill_switch::{KillSwitchManager, KillSwitchState};
use stellarroute_routing::chain_asset::ChainAsset;
use stellarroute_routing::compaction::CompactedGraph;
use stellarroute_routing::cross_chain::ProviderPolicy;
use stellarroute_routing::health::policy::OverrideDirective;
use stellarroute_routing::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig};
use stellarroute_routing::policy::RoutingPolicy;

const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

#[tokio::test]
async fn disabled_provider_cannot_be_selected_via_kill_switch_policy() {
    let manager = KillSwitchManager::new(None);
    let mut providers = HashMap::new();
    providers.insert("bad-dex".into(), OverrideDirective::ForceExclude);
    manager
        .update_state(KillSwitchState {
            providers,
            ..Default::default()
        })
        .await
        .unwrap();

    let provider_policy: ProviderPolicy = manager.get_provider_policy().await;
    assert!(!provider_policy.is_provider_allowed(Some("bad-dex")));

    // Mirror production construction: default policy + kill-switch providers,
    // bridges never opted in.
    let policy = RoutingPolicy::default().with_provider_policy(provider_policy);
    assert!(!policy.allow_bridge_edges);

    let xlm = ChainAsset::stellar_native("pubnet").to_canonical();
    let usdc = ChainAsset::stellar_credit("pubnet", "USDC", ISSUER)
        .unwrap()
        .to_canonical();

    let edges = vec![
        LiquidityEdge {
            from: xlm.clone(),
            to: usdc.clone(),
            venue_type: "amm".into(),
            venue_ref: "pool-bad".into(),
            liquidity: 10_000_000_000,
            price: 1.0,
            fee_bps: 30,
            provider: Some("bad-dex".into()),
            bridge: None,
        },
        LiquidityEdge {
            from: xlm.clone(),
            to: usdc.clone(),
            venue_type: "amm".into(),
            venue_ref: "pool-good".into(),
            liquidity: 10_000_000_000,
            price: 1.0,
            fee_bps: 30,
            provider: Some("good-dex".into()),
            bridge: None,
        },
    ];

    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let paths = finder
        .find_paths(&xlm, &usdc, &edges, 1_000_000, &policy)
        .expect("good-dex should remain selectable");
    assert!(!paths.is_empty());
    for path in &paths {
        for hop in &path.hops {
            assert_ne!(hop.provider.as_deref(), Some("bad-dex"));
            assert_ne!(hop.venue_type, "bridge");
        }
    }
}

/// Mirrors `routes_endpoint::routes_routing_policy` used by production + canary.
#[test]
fn routes_and_canary_policy_construction_preserves_provider_policy() {
    let provider_policy = ProviderPolicy::default()
        .with_kill_switch("bad-dex", true)
        .with_denylist(vec!["also-bad".into()]);
    let policy = RoutingPolicy {
        max_hops: 3,
        allow_bridge_edges: false,
        provider_policy: provider_policy.clone(),
        ..Default::default()
    };
    assert!(!policy.allow_bridge_edges);
    assert!(!policy.is_provider_allowed(Some("bad-dex")));
    assert!(!policy.is_provider_allowed(Some("also-bad")));
    assert!(policy.is_provider_allowed(Some("good-dex")));
    assert_eq!(
        policy.provider_policy.kill_switches.get("bad-dex"),
        provider_policy.kill_switches.get("bad-dex")
    );
}

#[test]
fn compacted_provider_edge_excluded_on_routes_style_policy() {
    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
    let xlm = ChainAsset::stellar_native("pubnet").to_canonical();
    let usdc = ChainAsset::stellar_credit("pubnet", "USDC", ISSUER)
        .unwrap()
        .to_canonical();
    let edges = vec![
        LiquidityEdge {
            from: xlm.clone(),
            to: usdc.clone(),
            venue_type: "amm".into(),
            venue_ref: "pool-bad".into(),
            liquidity: 10_000_000_000,
            price: 1.0,
            fee_bps: 30,
            provider: Some("bad-dex".into()),
            bridge: None,
        },
        LiquidityEdge {
            from: xlm.clone(),
            to: usdc.clone(),
            venue_type: "amm".into(),
            venue_ref: "pool-good".into(),
            liquidity: 10_000_000_000,
            price: 1.0,
            fee_bps: 30,
            provider: Some("good-dex".into()),
            bridge: None,
        },
    ];
    let restored = CompactedGraph::from_edges(edges).to_edges();
    let policy = RoutingPolicy {
        max_hops: 3,
        allow_bridge_edges: false,
        provider_policy: ProviderPolicy::default().with_kill_switch("bad-dex", true),
        ..Default::default()
    };
    let finder = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    });
    let paths = finder
        .find_paths(&xlm, &usdc, &restored, 1_000_000, &policy)
        .unwrap();
    for path in paths {
        for hop in path.hops {
            assert_ne!(hop.provider.as_deref(), Some("bad-dex"));
        }
    }
}
