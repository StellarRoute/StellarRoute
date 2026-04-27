//! StellarRoute API Server
//!
//! Provides REST API endpoints for price quotes and orderbook data.

pub mod audit;
pub mod cache;
pub mod docs;
pub mod error;
pub mod exactlyonce;
pub mod graph;
pub mod handlers;
pub mod indexer_lag;
pub mod kill_switch;
pub mod load_test;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod regions;
pub mod replay;
pub mod routes;
pub mod server;
pub mod shutdown;
pub mod state;
pub mod telemetry;
pub mod tracing_config;
pub mod worker;

pub use cache::CacheManager;
pub use docs::ApiDoc;
pub use error::{ApiError, Result};
pub use exactlyonce::{DedupeLedger, ExactlyOnceError, RequestIdentity};
pub use server::{Server, ServerConfig};
pub use state::AppState;
pub use tracing_config::{TraceContext, TracingConfig};
