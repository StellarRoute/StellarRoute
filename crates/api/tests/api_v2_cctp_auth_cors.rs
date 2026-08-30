//! Auth/CORS integration: CCTP bridge exempt from global API-key gate.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use stellarroute_api::{
    cctp::{
        bootstrap::CctpHttpContext,
        config::CctpConfig,
        idempotency::InMemoryCctpQuoteIdempotencyStore,
        iris::{IrisClient, IrisFeeQuote, IrisPollOutcome},
        prepare_lock::InMemoryCctpPrepareLockStore,
        readiness::CctpRuntime,
        service::CctpService,
        store::{CctpTransferStore, InMemoryCctpTransferStore},
    },
    kill_switch::KillSwitchManager,
    middleware::{AuthConfig, AuthLayer},
    routes,
    state::{AppState, DatabasePools},
};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct MockIris;

#[async_trait::async_trait]
impl IrisClient for MockIris {
    async fn fetch_burn_fees(
        &self,
        _: u32,
        _: u32,
    ) -> Result<stellarroute_api::cctp::iris::IrisFeeQuote, stellarroute_api::cctp::iris::IrisError>
    {
        Ok(IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        })
    }

    async fn poll_messages_by_tx(
        &self,
        _: u32,
        _: &str,
    ) -> Result<IrisPollOutcome, stellarroute_api::cctp::iris::IrisError> {
        Ok(IrisPollOutcome::Pending)
    }

    async fn reattest(&self, _: &str) -> Result<(), stellarroute_api::cctp::iris::IrisError> {
        Ok(())
    }
}

fn enabled_test_context() -> Arc<CctpHttpContext> {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    let runtime = CctpRuntime::production_defaults();
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let idempotency = Arc::new(InMemoryCctpQuoteIdempotencyStore::default());
    idempotency.bind_transfer_store(store.clone());
    let service = Arc::new(CctpService {
        config: cfg.clone(),
        store,
        prepare_lock: Arc::new(InMemoryCctpPrepareLockStore::default()),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: runtime.clone(),
    });
    Arc::new(CctpHttpContext {
        config: cfg,
        service,
        runtime,
        idempotency,
        access_token_keys: stellarroute_api::cctp::access::test_access_token_keyring(),
    })
}

async fn authed_router(ctx: Option<Arc<CctpHttpContext>>) -> axum::Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .expect("lazy pool");
    let mut state = AppState::new(DatabasePools::new(pool, None));
    if let Some(ctx) = ctx {
        state = state.with_cctp(ctx);
    }
    let router = routes::create_router(state.into_arc());
    let auth = AuthConfig {
        valid_keys: Arc::new(["secret-key".to_string()].into_iter().collect()),
        require_auth: true,
        public_get_routes: Arc::new(Default::default()),
    };
    router.layer(AuthLayer::new(auth))
}

#[tokio::test]
async fn cctp_quote_bypasses_require_auth_while_admin_stays_protected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let router = authed_router(Some(enabled_test_context())).await;

    let quote = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/bridge/cctp/quote")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "corridor_id":"circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
                        "provider":"circle-cctp",
                        "direction":"stellar_to_evm",
                        "source_chain_id":"stellar:testnet",
                        "destination_chain_id":"eip155:11155111",
                        "source_asset":{"chain_id":"stellar:testnet","asset":"erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA","canonical":"stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"},
                        "destination_asset":{"chain_id":"eip155:11155111","asset":"erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238","canonical":"eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"},
                        "amount":"10.0000000",
                        "recipient":"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                        "sender":"GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                        "finality":"standard"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(quote.status(), StatusCode::UNAUTHORIZED);

    let protected = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(protected.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn adjacent_v2_post_still_requires_api_key() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let router = authed_router(None).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/assets/canonicalize")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"chain_id":"stellar:testnet","asset":"native"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "unauthorized");
}

#[tokio::test]
async fn get_api_v2_metadata_is_public_without_api_key() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let router = authed_router(Some(enabled_test_context())).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cctp_bridge_traversal_paths_stay_protected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let router = authed_router(Some(enabled_test_context())).await;
    for path in [
        "/api/v2/bridge/cctp/../admin",
        "/api/v2/bridge/cctp/%2e%2e/admin",
        "/api/v2/bridge/cctp/not-a-uuid/prepare-burn",
        "/api/v2/bridge/cctp/550e8400-e29b-41d4-a716-446655440000/extra",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "expected 401 for {path}"
        );
    }
}
