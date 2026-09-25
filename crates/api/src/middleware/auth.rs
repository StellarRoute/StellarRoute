use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashSet, sync::Arc};
use tower::{Layer, Service};
use tracing::warn;

use crate::models::{ApiErrorCode, ErrorResponse};

/// Resolve whether API-key authentication is required.
///
/// `REQUIRE_AUTH`, when explicitly set, always wins. When unset, the
/// default is derived from the deployment profile: `true` in production
/// (`STELLARROUTE_ENV=production`), `false` everywhere else (dev/test).
pub fn resolve_require_auth() -> bool {
    match std::env::var("REQUIRE_AUTH") {
        Ok(v) => crate::env_profile::parse_bool(&v),
        Err(_) => crate::env_profile::is_production(),
    }
}

/// Startup guard for the production auth posture.
///
/// Returns:
/// - `Ok(None)` — nothing to report, boot normally.
/// - `Ok(Some(warning))` — production is running with auth disabled under an
///   explicit break-glass override (`ALLOW_INSECURE_PUBLIC_API=1`); log the
///   warning and continue booting.
/// - `Err(message)` — production is running with auth disabled and no
///   break-glass override was set; refuse to boot.
pub fn validate_auth_startup() -> Result<Option<String>, String> {
    if !crate::env_profile::is_production() || resolve_require_auth() {
        return Ok(None);
    }

    if crate::env_profile::parse_bool_env("ALLOW_INSECURE_PUBLIC_API") {
        Ok(Some(
            "STELLARROUTE_ENV=production is running with REQUIRE_AUTH disabled because \
             ALLOW_INSECURE_PUBLIC_API=1 was set. This is a break-glass override — quote/replay \
             surfaces are reachable without an API key. If only reads should be public, prefer \
             PUBLIC_GET_ROUTES over disabling auth globally."
                .to_string(),
        ))
    } else {
        Err(
            "Refusing to start: STELLARROUTE_ENV=production requires authentication \
             (REQUIRE_AUTH defaults to true in production). Set REQUIRE_AUTH=true, or if you \
             understand the risk and intend to run without authentication, set \
             ALLOW_INSECURE_PUBLIC_API=1 explicitly."
                .to_string(),
        )
    }
}

/// Paths that are always exempt from the global API-key gate, regardless of
/// `REQUIRE_AUTH` or method, because they carry their own dedicated access
/// control:
/// - `/health`, `/health/deps` — carry no sensitive data and must stay
///   reachable for load balancers / orchestrators in every profile.
/// - `/metrics`, `/api/v1/replay` (list/get/run/diff) — gated by
///   `production_admin_guard` (`ADMIN_AUTH_TOKEN`) in production instead;
///   see `docs/api/production-exposure.md`.
/// - `/api/v1/admin/*`, `/api/v1/system/*` — every route under these
///   prefixes already requires `AdminAuth` directly (mutations, always) or
///   via `production_admin_guard` (read-only state, production only); see
///   issues #1053, #1055, #1058.
///
/// Without this exemption, these routes would need both an API key *and* an
/// admin token in production, which none of the above tickets ask for.
const ALWAYS_EXEMPT_PREFIXES: &[&str] = &[
    "/health",
    "/metrics",
    "/api/v1/replay",
    "/api/v1/admin",
    "/api/v1/system",
];

/// Narrow CCTP bridge exemption: browser clients authenticate transfers via
/// `x-cctp-transfer-access` instead of integrator API keys. Quote remains
/// unauthenticated but route-level rate limited.
fn is_always_exempt(path: &str) -> bool {
    ALWAYS_EXEMPT_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn is_public_v2_metadata(method: &axum::http::Method, path: &str) -> bool {
    method == axum::http::Method::GET && path == "/api/v2"
}

const CCTP_BRIDGE_PREFIX: &str = "/api/v2/bridge/cctp/";
const CCTP_QUOTE_PATH: &str = "/api/v2/bridge/cctp/quote";

/// Classic browser swap flow: prepare returns unsigned XDR; submit requires a
/// wallet-signed envelope bound to `quote_id`. No integrator API key is used by
/// the public frontend — authorization is the Stellar signature + sender lock.
const CLASSIC_SWAP_PREPARE_PATH: &str = "/api/v1/swap/prepare";
const CLASSIC_SWAP_SUBMIT_PATH: &str = "/api/v1/swap/submit";

fn is_classic_swap_exempt(method: &axum::http::Method, path: &str) -> bool {
    *method == axum::http::Method::POST
        && (path == CLASSIC_SWAP_PREPARE_PATH || path == CLASSIC_SWAP_SUBMIT_PATH)
}

fn is_valid_uuid(segment: &str) -> bool {
    if segment.len() != 36 {
        return false;
    }
    let bytes = segment.as_bytes();
    bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && segment.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn is_cctp_bridge_exempt(method: &axum::http::Method, path: &str) -> bool {
    if path.contains("..") || path.contains("//") || path.contains('%') || path.contains('.') {
        return false;
    }
    if *method == axum::http::Method::POST && path == CCTP_QUOTE_PATH {
        return true;
    }
    if !path.starts_with(CCTP_BRIDGE_PREFIX) {
        return false;
    }
    let rest = &path[CCTP_BRIDGE_PREFIX.len()..];
    if rest.is_empty() || rest.starts_with('/') {
        return false;
    }
    if rest == "quote" {
        return false;
    }
    let (uuid_part, suffix) = match rest.split_once('/') {
        None => (rest, ""),
        Some((id, suf)) => (id, suf),
    };
    if !is_valid_uuid(uuid_part) {
        return false;
    }
    match suffix {
        "" => *method == axum::http::Method::GET,
        "prepare-burn" | "submit-burn" | "prepare-mint" | "submit-mint" | "reattest" => {
            *method == axum::http::Method::POST
        }
        _ => false,
    }
}

/// Parse `PUBLIC_GET_ROUTES` as a CSV allowlist of route path prefixes that
/// remain reachable via unauthenticated `GET` requests even when
/// `REQUIRE_AUTH` is enabled (e.g. public quote/orderbook reads for a
/// browser frontend, while admin/system routes still require a key).
fn public_get_routes_from_env() -> HashSet<String> {
    std::env::var("PUBLIC_GET_ROUTES")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Clone)]
pub struct AuthConfig {
    pub valid_keys: Arc<HashSet<String>>,
    pub require_auth: bool,
    /// Route path prefixes exempt from auth for `GET` requests (explicit
    /// public-read allowlist; never bypasses non-GET methods).
    pub public_get_routes: Arc<HashSet<String>>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let keys_env = std::env::var("API_KEYS").unwrap_or_default();
        let valid_keys: HashSet<String> = keys_env
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            valid_keys: Arc::new(valid_keys),
            require_auth: resolve_require_auth(),
            public_get_routes: Arc::new(public_get_routes_from_env()),
        }
    }
}

#[derive(Clone)]
pub struct AuthLayer {
    config: AuthConfig,
}

impl AuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }
}

impl Default for AuthLayer {
    fn default() -> Self {
        Self::new(AuthConfig::default())
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    config: AuthConfig,
}

fn is_public_get(config: &AuthConfig, req: &Request) -> bool {
    if req.method() != axum::http::Method::GET {
        return false;
    }
    let path = req.uri().path();
    config
        .public_get_routes
        .iter()
        .any(|prefix| path.starts_with(prefix.as_str()))
}

impl<S> Service<Request> for AuthService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // CORS preflight must reach the CORS layer. Auth sits outside CORS in
            // some deployments; never 401 OPTIONS or browsers report "Failed to fetch".
            if req.method() == axum::http::Method::OPTIONS {
                return inner.call(req).await;
            }

            if is_always_exempt(req.uri().path()) {
                return inner.call(req).await;
            }

            if is_public_v2_metadata(req.method(), req.uri().path()) {
                return inner.call(req).await;
            }

            if is_cctp_bridge_exempt(req.method(), req.uri().path()) {
                return inner.call(req).await;
            }

            if is_classic_swap_exempt(req.method(), req.uri().path()) {
                return inner.call(req).await;
            }

            if !config.require_auth && config.valid_keys.is_empty() {
                return inner.call(req).await;
            }

            if config.require_auth && is_public_get(&config, &req) {
                return inner.call(req).await;
            }

            let api_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());
            let bearer_token = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            let provided_key = api_key.or(bearer_token);

            match provided_key {
                Some(key) if config.valid_keys.contains(key) => inner.call(req).await,
                Some(_) => {
                    warn!("Invalid API key provided");
                    let response = (
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse::new(
                            ApiErrorCode::Unauthorized,
                            "Invalid API key provided",
                        )),
                    )
                        .into_response();
                    Ok(response)
                }
                None => {
                    if config.require_auth {
                        warn!("Missing API key");
                        let response = (
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse::new(
                                ApiErrorCode::Unauthorized,
                                "API key is required",
                            )),
                        )
                            .into_response();
                        Ok(response)
                    } else {
                        inner.call(req).await
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        std::env::remove_var("STELLARROUTE_ENV");
        std::env::remove_var("REQUIRE_AUTH");
        std::env::remove_var("ALLOW_INSECURE_PUBLIC_API");
        std::env::remove_var("PUBLIC_GET_ROUTES");
    }

    #[test]
    fn always_exempt_covers_health_metrics_replay_admin_and_system() {
        assert!(is_always_exempt("/health"));
        assert!(is_always_exempt("/health/deps"));
        assert!(is_always_exempt("/metrics"));
        assert!(is_always_exempt("/metrics/cache"));
        assert!(is_always_exempt("/metrics/pool"));
        assert!(is_always_exempt("/api/v1/replay"));
        assert!(is_always_exempt("/api/v1/replay/abc/run"));
        assert!(is_always_exempt("/api/v1/admin/kill-switch"));
        assert!(is_always_exempt("/api/v1/admin/cache/flush"));
        assert!(is_always_exempt("/api/v1/system/canary/report"));
        assert!(is_always_exempt("/api/v1/system/canary/config"));
        assert!(!is_always_exempt("/api/v1/quote/XLM/USDC"));
    }

    #[test]
    fn require_auth_defaults_false_in_dev() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        assert!(!resolve_require_auth());
    }

    #[test]
    fn require_auth_defaults_true_in_production() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        let result = resolve_require_auth();
        reset_env();
        assert!(result);
    }

    #[test]
    fn require_auth_explicit_false_overrides_production_default() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("REQUIRE_AUTH", "false");
        let result = resolve_require_auth();
        reset_env();
        assert!(!result);
    }

    #[test]
    fn require_auth_explicit_true_in_dev() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("REQUIRE_AUTH", "true");
        let result = resolve_require_auth();
        reset_env();
        assert!(result);
    }

    #[test]
    fn startup_guard_refuses_boot_in_production_without_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("REQUIRE_AUTH", "false");
        let result = validate_auth_startup();
        reset_env();
        assert!(result.is_err());
    }

    #[test]
    fn startup_guard_warns_with_break_glass_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("REQUIRE_AUTH", "false");
        std::env::set_var("ALLOW_INSECURE_PUBLIC_API", "1");
        let result = validate_auth_startup();
        reset_env();
        assert!(matches!(result, Ok(Some(_))));
    }

    #[test]
    fn startup_guard_ok_in_production_with_auth_required() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        let result = validate_auth_startup();
        reset_env();
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn startup_guard_ok_outside_production() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("REQUIRE_AUTH", "false");
        let result = validate_auth_startup();
        reset_env();
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn cctp_bridge_paths_are_auth_exempt() {
        use axum::http::Method;
        assert!(is_cctp_bridge_exempt(
            &Method::POST,
            "/api/v2/bridge/cctp/quote"
        ));
        assert!(is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp/550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(is_cctp_bridge_exempt(
            &Method::POST,
            "/api/v2/bridge/cctp/550e8400-e29b-41d4-a716-446655440000/prepare-burn"
        ));
        assert!(!is_cctp_bridge_exempt(&Method::GET, "/api/v2"));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/assets/canonicalize"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v1/admin/kill-switch"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp/../admin"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp//550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp/%2e%2e/admin"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp/not-a-uuid/prepare-burn"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::GET,
            "/api/v2/bridge/cctp/550e8400-e29b-41d4-a716-446655440000/extra"
        ));
        assert!(!is_cctp_bridge_exempt(
            &Method::POST,
            "/api/v2/bridge/cctp/550e8400-e29b-41d4-a716-446655440000"
        ));
    }

    #[test]
    fn public_v2_metadata_is_auth_exempt() {
        use axum::http::Method;
        assert!(is_public_v2_metadata(&Method::GET, "/api/v2"));
        assert!(!is_public_v2_metadata(&Method::POST, "/api/v2"));
        assert!(!is_public_v2_metadata(&Method::GET, "/api/v2/"));
        assert!(!is_public_v2_metadata(&Method::GET, "/api/v2/assets"));
    }

    #[test]
    fn classic_swap_prepare_and_submit_are_auth_exempt() {
        use axum::http::Method;
        assert!(is_classic_swap_exempt(
            &Method::POST,
            "/api/v1/swap/prepare"
        ));
        assert!(is_classic_swap_exempt(&Method::POST, "/api/v1/swap/submit"));
        assert!(!is_classic_swap_exempt(
            &Method::GET,
            "/api/v1/swap/prepare"
        ));
        assert!(!is_classic_swap_exempt(
            &Method::POST,
            "/api/v1/swap/prepare/extra"
        ));
        assert!(!is_classic_swap_exempt(&Method::POST, "/api/v1/quote"));
    }
}
