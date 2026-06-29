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

#[derive(Clone)]
pub struct AuthConfig {
    pub valid_keys: Arc<HashSet<String>>,
    pub require_auth: bool,
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
            require_auth: std::env::var("REQUIRE_AUTH").unwrap_or_else(|_| "false".to_string())
                == "true",
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
            if !config.require_auth && config.valid_keys.is_empty() {
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
