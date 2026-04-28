//! Error types for routing

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("No route found from {0} to {1}")]
    NoRoute(String, String),

    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Invalid asset pair: {0}")]
    InvalidPair(String),

    #[error("Normalization failed: {0}")]
    Normalization(String),

    #[error("Decimal precision error: {0}")]
    DecimalPrecision(String),

    #[error("Numeric overflow during normalization")]
    Overflow,

    #[error("Duplicate scorer registered: {0}")]
    DuplicateScorer(String),

    #[error("Unknown scorer: {0}")]
    UnknownScorer(String),
}

pub type Result<T> = std::result::Result<T, RoutingError>;
