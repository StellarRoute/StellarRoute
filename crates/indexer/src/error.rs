//! Error types for the indexer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Stellar API error: {0}")]
    StellarApi(String),

    #[error("Soroban error: {0}")]
    Soroban(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Invalid asset: {0}")]
    InvalidAsset(String),

    #[error("Sync error: {0}")]
    Sync(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
