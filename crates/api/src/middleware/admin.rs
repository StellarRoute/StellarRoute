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

fn extract_admin_token(headers: &HeaderMap) -> Option<String> {
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
