//! Regression suite (issue #1058): every mutating (POST/PUT/DELETE) route
//! registered under `/api/v1/admin/*` or `/api/v1/system/*` must reject
//! unauthenticated requests with `401`/`403`.
//!
//! ## How to add a new admin/system mutating route to this suite
//!
//! Add an entry to `ADMIN_SYSTEM_MUTATING_ROUTES` below: the HTTP method and
//! a concrete request path (substitute real values for any axum `:param`
//! segments). `admin_system_mutating_routes_are_all_registered_in_table`
//! scans `crates/api/src/routes/mod.rs` for `.route(...)` registrations
//! under `/api/v1/admin` / `/api/v1/system` with a `post`/`put`/`delete`
//! handler and fails the build if one isn't covered here — so a new
//! mutating route that forgets to update this table cannot silently ship
//! without auth coverage.
//!
//! Runs fully in-process against a lazily-connected Postgres pool (never
//! actually dials out), so it requires no network access.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

/// (HTTP method, concrete request path to call in the regression test).
const ADMIN_SYSTEM_MUTATING_ROUTES: &[(Method, &str)] = &[
    (Method::POST, "/api/v1/admin/cache/flush/XLM/USDC"),
    (Method::POST, "/api/v1/admin/cache/flush"),
    (Method::POST, "/api/v1/admin/kill-switch"),
    (Method::POST, "/api/v1/system/canary/config"),
];

async fn setup_router() -> axum::Router {
    // No ADMIN_AUTH_TOKEN configured: admin routes must deny by default.
    std::env::remove_var("ADMIN_AUTH_TOKEN");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    let mut config = ServerConfig::default();
    config.admin_auth_token = None;

    Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn unauthenticated_admin_and_system_mutations_are_denied() {
    let router = setup_router().await;

    for (method, path) in ADMIN_SYSTEM_MUTATING_ROUTES {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(*path)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert!(
            response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN,
            "expected 401/403 for unauthenticated {method} {path}, got {}",
            response.status()
        );
    }
}

/// Extract `(METHOD, router_path)` pairs for every `post`/`put`/`delete`
/// `.route(...)` registration under `/api/v1/admin` or `/api/v1/system` in
/// the given router source.
fn extract_admin_system_mutating_routes_from_source(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();

    for chunk in source.split(".route(").skip(1) {
        let Some(q1) = chunk.find('"') else {
            continue;
        };
        let rest = &chunk[q1 + 1..];
        let Some(q2) = rest.find('"') else {
            continue;
        };
        let path = &rest[..q2];
        if !(path.starts_with("/api/v1/admin/") || path.starts_with("/api/v1/system/")) {
            continue;
        }

        let after_path = &rest[q2 + 1..];
        let window_len = after_path.len().min(200);
        let verb_window = &after_path[..window_len];

        let verb = ["post", "put", "delete", "patch", "get"]
            .iter()
            .filter_map(|v| verb_window.find(&format!("{v}(")).map(|idx| (idx, *v)))
            .min_by_key(|(idx, _)| *idx)
            .map(|(_, v)| v);

        if let Some(verb) = verb {
            if verb != "get" {
                found.push((verb.to_uppercase(), path.to_string()));
            }
        }
    }

    found
}

/// Whether a concrete request path (real values substituted in) matches an
/// axum router path (which may contain `:param` segments).
fn paths_match(router_path: &str, concrete_path: &str) -> bool {
    let router_segments: Vec<&str> = router_path.split('/').collect();
    let concrete_segments: Vec<&str> = concrete_path.split('/').collect();
    if router_segments.len() != concrete_segments.len() {
        return false;
    }
    router_segments
        .iter()
        .zip(concrete_segments.iter())
        .all(|(r, c)| r.starts_with(':') || r == c)
}

#[test]
fn admin_system_mutating_routes_are_all_registered_in_table() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/routes/mod.rs"));
    let discovered = extract_admin_system_mutating_routes_from_source(source);

    assert!(
        !discovered.is_empty(),
        "route scanner found zero admin/system mutating routes in routes/mod.rs; \
         the scanner likely broke (check extract_admin_system_mutating_routes_from_source)"
    );

    for (method, router_path) in &discovered {
        let covered = ADMIN_SYSTEM_MUTATING_ROUTES
            .iter()
            .any(|(m, concrete_path)| m.as_str() == method && paths_match(router_path, concrete_path));

        assert!(
            covered,
            "found mutating route {method} {router_path} registered under /api/v1/admin or \
             /api/v1/system in routes/mod.rs, but ADMIN_SYSTEM_MUTATING_ROUTES in \
             tests/unauthenticated_admin_mutations.rs has no matching entry. Add one (and make \
             sure the handler requires AdminAuth) before merging."
        );
    }
}
