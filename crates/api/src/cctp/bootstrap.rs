//! Production CCTP service bootstrap from shared DB pool + async runtime wiring.

use std::sync::Arc;

use sqlx::PgPool;
use tracing::warn;

use crate::cctp::access::CctpAccessTokenKeyRing;
use crate::cctp::config::CctpConfig;
use crate::cctp::idempotency::PgCctpQuoteIdempotencyStore;
use crate::cctp::iris::ReqwestIrisClient;
use crate::cctp::prepare_lock::PgCctpPrepareLockStore;
use crate::cctp::readiness::CctpRuntime;
use crate::cctp::service::CctpService;
use crate::cctp::store::PgCctpTransferStore;
use crate::kill_switch::KillSwitchManager;
use crate::metrics;

/// Shared HTTP-facing CCTP dependencies constructed once at server startup.
pub struct CctpHttpContext {
    pub config: CctpConfig,
    pub service: Arc<CctpService>,
    pub runtime: CctpRuntime,
    pub idempotency: Arc<dyn crate::cctp::idempotency::CctpQuoteIdempotencyStore>,
    pub access_token_keys: CctpAccessTokenKeyRing,
}

impl CctpHttpContext {
    pub async fn try_build(pool: PgPool, kill_switch: Arc<KillSwitchManager>) -> Option<Self> {
        let config = match CctpConfig::from_env() {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(error = %e, "CCTP config invalid; public bridge remains disabled");
                metrics::record_cctp_endpoint_outcome("bootstrap", "config_invalid");
                return None;
            }
        };

        let access_token_keys = match CctpAccessTokenKeyRing::from_env_when_enabled(config.enabled)
        {
            Ok(Some(keys)) => keys,
            Ok(None) => CctpAccessTokenKeyRing::from_single_key(vec![0u8; 32]),
            Err(e) => {
                warn!(error = %e, "CCTP access token HMAC key missing or weak");
                metrics::record_cctp_endpoint_outcome("bootstrap", "access_key_invalid");
                return None;
            }
        };

        if config.enabled && !config.is_configured() {
            warn!("CCTP enabled but not fully configured; public bridge remains disabled");
            metrics::record_cctp_endpoint_outcome("bootstrap", "not_configured");
            return None;
        }

        let runtime = CctpRuntime::from_config_async(&config).await;
        let iris = match ReqwestIrisClient::from_config(&config) {
            Ok(client) => Arc::new(client),
            Err(e) => {
                warn!(error = %e, "CCTP Iris client failed to initialize");
                metrics::record_cctp_endpoint_outcome("bootstrap", "iris_unavailable");
                return None;
            }
        };

        let store = Arc::new(PgCctpTransferStore::new(pool.clone()));
        let prepare_lock = Arc::new(PgCctpPrepareLockStore::new(pool.clone()));
        let idempotency: Arc<dyn crate::cctp::idempotency::CctpQuoteIdempotencyStore> =
            Arc::new(PgCctpQuoteIdempotencyStore::new(pool));

        let service = Arc::new(CctpService {
            config: config.clone(),
            store,
            prepare_lock,
            iris,
            kill_switch,
            runtime: runtime.clone(),
        });

        log_readiness(&config, &runtime);
        Some(Self {
            config,
            service,
            runtime,
            idempotency,
            access_token_keys,
        })
    }
}

fn log_readiness(config: &CctpConfig, runtime: &CctpRuntime) {
    use crate::models::v2_cctp::CctpDirection;

    for direction in [CctpDirection::StellarToEvm, CctpDirection::EvmToStellar] {
        let readiness = runtime.assess(direction);
        let label = match direction {
            CctpDirection::StellarToEvm => "stellar_to_evm",
            CctpDirection::EvmToStellar => "evm_to_stellar",
        };
        if readiness.is_ready() {
            metrics::record_cctp_direction_readiness(label, "ready");
        } else {
            metrics::record_cctp_direction_readiness(label, "not_ready");
            warn!(
                corridor = %config.corridor_id(),
                direction = label,
                missing = ?readiness.missing,
                enabled = config.enabled,
                "CCTP direction not publicly executable"
            );
        }
    }
}
