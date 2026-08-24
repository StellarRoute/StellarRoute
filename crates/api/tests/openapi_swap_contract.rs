//! OpenAPI + error-taxonomy contract tests for the swap prepare/submit
//! endpoints (issue #1051).
//!
//! Two things are verified here:
//! 1. `/api/v1/swap/prepare` and `/api/v1/swap/submit` are documented in the
//!    generated OpenAPI spec, under the `swap` tag, with request bodies and
//!    responses.
//! 2. Every `ApiErrorCode` variant (`ApiErrorCode::ALL`, the single source of
//!    truth in `crates/api/src/models/response.rs`) is mentioned in both
//!    `docs/api/error_taxonomy.md` and sdk-js's `API_ERROR_CODES`
//!    (`sdk-js/src/types.ts`). Adding a new `ApiErrorCode` variant without
//!    updating both of those files fails this test — that's the drift check
//!    the issue asks for ("lightweight fixture test acceptable").
//!
//! Runs fully in-process against a lazily-connected Postgres pool (never
//! actually dials out) and reads two source files as string fixtures, so it
//! requires no network access and no live database.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{models::ApiErrorCode, state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn openapi_spec_documents_swap_prepare_and_submit_under_swap_tag() {
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&body).unwrap();

    for (path, method) in [
        ("/api/v1/swap/prepare", "post"),
        ("/api/v1/swap/submit", "post"),
    ] {
        let operation = &spec["paths"][path][method];
        assert!(
            !operation.is_null(),
            "{method} {path} must be documented in the OpenAPI spec"
        );

        assert!(
            !operation["requestBody"].is_null(),
            "{method} {path} must document a requestBody"
        );
        assert!(
            operation["responses"]["200"].is_object(),
            "{method} {path} must document a 200 response"
        );

        let tags = operation["tags"]
            .as_array()
            .unwrap_or_else(|| panic!("{method} {path} must have a tags array"));
        assert!(
            tags.iter().any(|t| t == "swap"),
            "{method} {path} must be tagged 'swap', got {tags:?}"
        );
    }

    let schemas = &spec["components"]["schemas"];
    for schema_name in [
        "AssetPath",
        "SwapPrepareRequest",
        "SwapPrepareResponse",
        "SwapSubmitRequest",
        "SwapSubmitResponse",
    ] {
        assert!(
            schemas[schema_name].is_object() || schemas[schema_name]["oneOf"].is_array(),
            "{schema_name} schema must be in components.schemas"
        );
    }

    let asset_path = &schemas["AssetPath"];
    let one_of = asset_path["oneOf"]
        .as_array()
        .expect("AssetPath OpenAPI schema must be oneOf string|object");
    assert!(
        one_of.len() >= 2,
        "AssetPath oneOf must include string and object variants, got {asset_path}"
    );
    assert!(
        one_of.iter().any(|item| item["type"] == "string"),
        "AssetPath oneOf must include a string variant, got {asset_path}"
    );
    assert!(
        one_of.iter().any(|item| {
            item["type"] == "object"
                || item["properties"]["asset_code"].is_object()
                || item["required"]
                    .as_array()
                    .is_some_and(|req| req.iter().any(|r| r == "asset_code"))
        }),
        "AssetPath oneOf must include an object variant with asset_code, got {asset_path}"
    );

    let prepare_props = &schemas["SwapPrepareResponse"]["properties"];
    assert!(
        prepare_props["network_passphrase"].is_object(),
        "SwapPrepareResponse must document network_passphrase, got {}",
        schemas["SwapPrepareResponse"]
    );
}

#[test]
fn openapi_error_taxonomy_matches_docs_and_sdk_js() {
    let taxonomy_doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/api/error_taxonomy.md"
    ));
    let sdk_types = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../sdk-js/src/types.ts"
    ));

    for code in ApiErrorCode::ALL {
        let code_str = code.as_str();

        assert!(
            taxonomy_doc.contains(&format!("`{code_str}`")),
            "docs/api/error_taxonomy.md is missing documented code `{code_str}` \
             (present in ApiErrorCode::ALL)"
        );

        assert!(
            sdk_types.contains(&format!("'{code_str}'")),
            "sdk-js/src/types.ts's API_ERROR_CODES is missing '{code_str}' \
             (present in ApiErrorCode::ALL) — sdk-js error code union has \
             drifted from the documented Rust taxonomy"
        );
    }
}
