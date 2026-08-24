//! Admin authentication extraction

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts, HeaderMap};
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

/// Extractor that verifies the admin authentication token.
pub struct AdminAuth;

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let expected_token = state
            .admin_auth_token
            .as_ref()
            .ok_or_else(|| ApiError::Unauthorized("Admin auth is not configured".to_string()))?;

        let token = extract_admin_token(&parts.headers).ok_or_else(|| {
            ApiError::Unauthorized("Missing admin authorization header".to_string())
        })?;

        if token != *expected_token {
            return Err(ApiError::Unauthorized(
                "Invalid admin credentials".to_string(),
            ));
        }

        Ok(AdminAuth)
    }
}

pub(crate) fn extract_admin_token(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-admin-token").and_then(|v| v.to_str().ok()) {
        return Some(value.trim().to_string());
    }

    if let Some(auth) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.trim().to_string());
        }
    }

    None
}

/// Startup guard (issues #1053, #1055): in production, `ADMIN_AUTH_TOKEN`
/// must be configured. Every admin/system mutation (`/api/v1/admin/*`,
/// `/api/v1/system/*`) already denies by default when it's unset — but that
/// means the API would boot into a state where legitimate operators can
/// never reach the kill switch or canary config either, silently. Refuse to
/// boot instead so the misconfiguration is caught immediately rather than
/// discovered during an incident.
pub fn validate_admin_auth_startup() -> Result<(), String> {
    if !crate::env_profile::is_production() {
        return Ok(());
    }

    let configured = std::env::var("ADMIN_AUTH_TOKEN")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    if configured {
        Ok(())
    } else {
        Err(
            "Refusing to start: STELLARROUTE_ENV=production requires ADMIN_AUTH_TOKEN to be \
             set. It gates /api/v1/admin/*, /api/v1/system/*, and (in production) /metrics + \
             /api/v1/replay/*. Set ADMIN_AUTH_TOKEN before starting in production."
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        std::env::remove_var("STELLARROUTE_ENV");
        std::env::remove_var("ADMIN_AUTH_TOKEN");
    }

    #[test]
    fn admin_auth_startup_ok_outside_production_without_token() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        let result = validate_admin_auth_startup();
        assert!(result.is_ok());
    }

    #[test]
    fn admin_auth_startup_refuses_boot_in_production_without_token() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        let result = validate_admin_auth_startup();
        reset_env();
        assert!(result.is_err());
    }

    #[test]
    fn admin_auth_startup_ok_in_production_with_token() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("ADMIN_AUTH_TOKEN", "some-token");
        let result = validate_admin_auth_startup();
        reset_env();
        assert!(result.is_ok());
    }
}
