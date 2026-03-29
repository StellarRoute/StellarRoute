//! StellarRoute API Server
//!
//! Provides REST API endpoints for price quotes and orderbook data.

pub mod cache;
pub mod docs;
pub mod error;
pub mod handlers;
pub mod load_test;
pub mod middleware;
pub mod models;
pub mod regions;
pub mod replay;
pub mod routes;
pub mod server;
pub mod state;
pub mod telemetry;
pub mod tracing_config;
pub mod worker;
pub mod graph;

pub use cache::CacheManager;
pub use docs::ApiDoc;
pub use error::{ApiError, Result};
pub use server::{Server, ServerConfig};
pub use state::AppState;
pub use tracing_config::{TracingConfig, TraceContext};
