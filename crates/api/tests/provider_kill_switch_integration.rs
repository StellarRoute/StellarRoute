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

// ── Default-off guards (issue #1283) ─────────────────────────────────────────
//
// The provider kill switch ships empty. These lock that an untouched manager
// leaves classic SDEX routing alone, and that flipping one provider removes
// only that provider.

/// A `KillSwitchManager` that was never updated must produce a permissive
/// provider policy — including for the SDEX provider that serves classic
/// one-hop quotes.
#[tokio::test]
async fn default_provider_policy_allows_sdex() {
    let manager = KillSwitchManager::new(None);
    let provider_policy: ProviderPolicy = manager.get_provider_policy().await;

    assert!(provider_policy.kill_switches.is_empty());
    assert!(provider_policy.is_provider_allowed(Some("sdex")));
    assert!(provider_policy.is_provider_allowed(Some("any-other-dex")));
    assert!(provider_policy.is_provider_allowed(None));

    let policy = RoutingPolicy::default().with_provider_policy(provider_policy);
    let (xlm, usdc) = xlm_usdc();
    let paths = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    })
    .find_paths(&xlm, &usdc, &[sdex_edge(&xlm, &usdc, "sdex:1", "sdex")], 1_000_000, &policy)
    .expect("default policy must not block SDEX");

    assert!(
        !paths.is_empty(),
        "default (empty) kill switch must leave SDEX routable"
    );
}

/// Enabling one provider must not take its siblings with it: a second SDEX
/// provider stays routable.
#[tokio::test]
async fn enabled_provider_switch_excludes_only_the_targeted_provider() {
    let manager = KillSwitchManager::new(None);
    let mut providers = HashMap::new();
    providers.insert("sdex-a".into(), OverrideDirective::ForceExclude);
    manager
        .update_state(KillSwitchState {
            providers,
            ..Default::default()
        })
        .await
        .unwrap();

    let provider_policy: ProviderPolicy = manager.get_provider_policy().await;
    assert!(!provider_policy.is_provider_allowed(Some("sdex-a")));
    assert!(provider_policy.is_provider_allowed(Some("sdex-b")));

    let policy = RoutingPolicy::default().with_provider_policy(provider_policy);
    let (xlm, usdc) = xlm_usdc();
    let edges = vec![
        sdex_edge(&xlm, &usdc, "sdex:a", "sdex-a"),
        sdex_edge(&xlm, &usdc, "sdex:b", "sdex-b"),
    ];

    let paths = Pathfinder::new(PathfinderConfig {
        min_liquidity_threshold: 1,
    })
    .find_paths(&xlm, &usdc, &edges, 1_000_000, &policy)
    .expect("sdex-b should remain selectable");

    assert!(!paths.is_empty(), "untargeted SDEX provider stays routable");
    for path in &paths {
        for hop in &path.hops {
            assert_ne!(hop.provider.as_deref(), Some("sdex-a"));
        }
    }
}

fn xlm_usdc() -> (String, String) {
    (
        ChainAsset::stellar_native("pubnet").to_canonical(),
        ChainAsset::stellar_credit("pubnet", "USDC", ISSUER)
            .unwrap()
            .to_canonical(),
    )
}

fn sdex_edge(from: &str, to: &str, venue_ref: &str, provider: &str) -> LiquidityEdge {
    LiquidityEdge {
        from: from.to_string(),
        to: to.to_string(),
        venue_type: "sdex".into(),
        venue_ref: venue_ref.into(),
        liquidity: 10_000_000_000,
        price: 1.0,
        fee_bps: 30,
        provider: Some(provider.into()),
        bridge: None,
    }
}
