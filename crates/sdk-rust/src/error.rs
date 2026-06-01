//! Error types for the StellarRoute Rust SDK.
//!
//! [`SdkError`] is the single error type returned by every client method.
//! It distinguishes transport failures, API-level errors (with typed error
//! codes), serialization problems, and configuration mistakes so callers can
//! handle each case precisely.

use thiserror::Error;

// ── Typed API error codes ─────────────────────────────────────────────────────
// These mirror the `error` field in the API's `ErrorResponse` schema.

/// Machine-readable error codes returned by the StellarRoute API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiErrorCode {
    /// Asset identifier could not be parsed (HTTP 400).
    InvalidAsset,
    /// A request parameter failed validation (HTTP 400).
    ValidationError,
    /// The requested resource does not exist (HTTP 404).
    NotFound,
    /// Too many requests from this IP (HTTP 429).
    RateLimitExceeded,
    /// The request was rejected because market data was stale (HTTP 422).
    StaleMarketData,
    /// The server is temporarily overloaded (HTTP 503).
    Overloaded,
    /// Unexpected server-side failure (HTTP 500).
    InternalError,
    /// Any other error code not listed above.
    Other(String),
}

impl ApiErrorCode {
    /// Returns `true` for a stale market data error.
    pub fn is_stale_market_data(&self) -> bool {
        matches!(self, Self::StaleMarketData)
    }

    /// Returns `true` for an overloaded service error.
    pub fn is_overloaded(&self) -> bool {
        matches!(self, Self::Overloaded)
    }
}

impl std::str::FromStr for ApiErrorCode {
    type Err = std::convert::Infallible;

    /// Parse the `error` string from an API error response.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "invalid_asset" => Self::InvalidAsset,
            "validation_error" => Self::ValidationError,
            "not_found" => Self::NotFound,
            "rate_limit_exceeded" => Self::RateLimitExceeded,
            "stale_market_data" => Self::StaleMarketData,
            "overloaded" => Self::Overloaded,
            "internal_error" => Self::InternalError,
            other => Self::Other(other.to_string()),
        })
    }
}

impl ApiErrorCode {
    /// Returns the canonical string representation used by the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::InvalidAsset => "invalid_asset",
            Self::ValidationError => "validation_error",
            Self::NotFound => "not_found",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::StaleMarketData => "stale_market_data",
            Self::Overloaded => "overloaded",
            Self::InternalError => "internal_error",
            Self::Other(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Rate-limit context ────────────────────────────────────────────────────────

/// Context extracted from `X-RateLimit-*` response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the current window.
    pub limit: Option<u32>,
    /// Requests remaining in the current window.
    pub remaining: Option<u32>,
    /// Unix timestamp when the window resets.
    pub reset: Option<u64>,
}

// ── Main error type ───────────────────────────────────────────────────────────

/// All errors that can be returned by the StellarRoute SDK.
#[derive(Error, Debug)]
pub enum SdkError {
    /// A transport-level failure (connection refused, timeout, TLS error, …).
    #[error("HTTP transport error: {0}")]
    Http(String),

    /// The API returned a non-2xx response with a structured error body.
    #[error("API error [{code}]: {message}")]
    Api {
        /// Typed error code from the `error` field of the response body.
        code: ApiErrorCode,
        /// Human-readable description from the `message` field.
        message: String,
        /// HTTP status code.
        status: u16,
    },

    /// The API returned a 429 with rate-limit headers.
    #[error("Rate limit exceeded (resets at {:?})", info.reset)]
    RateLimited {
        /// Parsed rate-limit header values.
        info: RateLimitInfo,
    },

    /// The response body could not be deserialized into the expected type.
    #[error("Failed to deserialize API response: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// The client was constructed with an invalid configuration value.
    #[error("Invalid SDK configuration: {0}")]
    InvalidConfig(String),
}

impl SdkError {
    /// Returns `true` if this is a transport-level error.
    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    /// Returns `true` if the API returned a 404 Not Found.
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::Api {
                code: ApiErrorCode::NotFound,
                ..
            }
        )
    }

    /// Returns `true` if the request was rate-limited.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimited { .. })
    }

    /// Returns `true` if the service is overloaded.
    pub fn is_overloaded(&self) -> bool {
        matches!(
            self,
            Self::Api {
                code: ApiErrorCode::Overloaded,
                ..
            }
        )
    }

    /// Returns `true` if the market data was stale.
    pub fn is_stale_market_data(&self) -> bool {
        matches!(
            self,
            Self::Api {
                code: ApiErrorCode::StaleMarketData,
                ..
            }
        )
    }

    /// Returns `true` if the request contained invalid parameters.
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            Self::Api {
                code: ApiErrorCode::ValidationError | ApiErrorCode::InvalidAsset,
                ..
            }
        )
    }

    /// Returns the HTTP status code if this is an API error.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::RateLimited { .. } => Some(429),
            _ => None,
        }
    }
}

/// Convenience alias used throughout the SDK.
pub type Result<T> = std::result::Result<T, SdkError>;
