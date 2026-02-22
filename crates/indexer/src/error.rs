//! Error types for the indexer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),

    #[error("Database query failed: {0}")]
    DatabaseQuery(#[from] sqlx::Error),

    #[error("Database migration failed: {0}")]
    DatabaseMigration(String),

    #[error("HTTP request failed: {url}, status: {status:?}, error: {error}")]
    HttpRequest {
        url: String,
        status: Option<u16>,
        error: String,
    },

    #[error("Network timeout after {timeout_secs}s: {context}")]
    NetworkTimeout { timeout_secs: u64, context: String },

    #[error("Network connection error: {0}")]
    NetworkConnection(String),

    #[error("API rate limit exceeded, retry after: {retry_after:?}s")]
    RateLimitExceeded { retry_after: Option<u64> },

    #[error("Stellar API error: {endpoint}, status: {status}, message: {message}")]
    StellarApi {
        endpoint: String,
        status: u16,
        message: String,
    },

    #[error("Invalid response from Stellar API: {0}")]
    StellarApiInvalidResponse(String),

    #[error("Soroban RPC error: {0}")]
    SorobanRpc(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Invalid configuration: {field}, reason: {reason}")]
    InvalidConfig { field: String, reason: String },

    #[error("Invalid asset: {asset}, reason: {reason}")]
    InvalidAsset { asset: String, reason: String },

    #[error("Invalid offer: {offer_id}, reason: {reason}")]
    InvalidOffer { offer_id: String, reason: String },

    #[error("JSON parsing error: {context}, error: {error}")]
    JsonParse { context: String, error: String },

    #[error("Numeric parsing error: {value}, expected: {expected_type}")]
    NumericParse {
        value: String,
        expected_type: String,
    },

    #[error("Missing required field: {field}, context: {context}")]
    MissingField { field: String, context: String },

    #[error("Synchronization error: {0}")]
    Sync(String),

    #[error("Indexer is not initialized")]
    NotInitialized,

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl IndexerError {
    pub fn log_level(&self) -> tracing::Level {
        use tracing::Level;
        match self {
            Self::DatabaseConnection(_) | Self::DatabaseMigration(_) => Level::ERROR,
            Self::NetworkConnection(_) | Self::HttpRequest { .. } => Level::WARN,
            Self::RateLimitExceeded { .. } => Level::WARN,
            Self::NetworkTimeout { .. } => Level::WARN,
            Self::Config(_) | Self::InvalidConfig { .. } => Level::ERROR,
            Self::JsonParse { .. } | Self::NumericParse { .. } => Level::WARN,
            Self::MissingField { .. } => Level::WARN,
            Self::InvalidAsset { .. } | Self::InvalidOffer { .. } => Level::WARN,
            Self::StellarApi { .. } | Self::StellarApiInvalidResponse(_) => Level::WARN,
            Self::DatabaseQuery(_) => Level::ERROR,
            _ => Level::ERROR,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NetworkTimeout { .. }
                | Self::NetworkConnection(_)
                | Self::RateLimitExceeded { .. }
                | Self::HttpRequest { .. }
        )
    }
}

impl From<reqwest::Error> for IndexerError {
    fn from(err: reqwest::Error) -> Self {
        let url = err.url().map(|u| u.to_string()).unwrap_or_default();
        let status = err.status().map(|s| s.as_u16());

        if err.is_timeout() {
            Self::NetworkTimeout {
                timeout_secs: 30,
                context: url,
            }
        } else if err.is_connect() {
            Self::NetworkConnection(format!("Failed to connect to {}: {}", url, err))
        } else {
            Self::HttpRequest {
                url,
                status,
                error: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for IndexerError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonParse {
            context: "JSON deserialization".to_string(),
            error: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, IndexerError>;

#[cfg(test)]
mod tests;
