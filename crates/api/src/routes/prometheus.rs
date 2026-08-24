//! Prometheus metrics endpoint

use axum::http::StatusCode;
use axum::response::Response;

/// Prometheus metrics endpoint
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "metrics",
    responses(
        (status = 200, description = "Prometheus metrics in text format", body = String),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn prometheus_metrics() -> Response<String> {
    match crate::metrics::encode_metrics() {
        Ok(metrics) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(metrics)
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("Error encoding metrics: {}", e))
            .unwrap(),
    }
}
