//! Country screening trait for card payments
//!
//! This module provides a `Screening` trait that validates whether a
//! transaction's country is allowed. The default implementation is
//! `AllowAllScreening` which permits all countries unless a denylist
//! is configured via `CARD_COUNTRY_DENYLIST`.

use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Error types for screening operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ScreeningError {
    #[error("Country {0} is blocked")]
    CountryBlocked(String),
    #[error("Screening service unavailable")]
    Unavailable,
}

/// Result type for screening operations
pub type ScreeningResult<T> = Result<T, ScreeningError>;

/// Trait for country screening implementations
///
/// Implementations should be stateless and thread-safe.
/// The trait is object-safe for dynamic dispatch.
#[async_trait]
pub trait Screening: Send + Sync {
    /// Check if a country code is allowed
    ///
    /// Returns `Ok(())` if the country is allowed, or `Err(ScreeningError::CountryBlocked)`
    /// if the country is on the denylist.
    async fn check_country(&self, country_code: &str) -> ScreeningResult<()>;

    /// Optional: Get the name of this screening implementation
    fn name(&self) -> &'static str {
        "Screening"
    }
}

/// Default screening implementation that allows all countries
/// unless they are on the configured denylist.
#[derive(Debug, Clone)]
pub struct AllowAllScreening {
    denylist: Vec<String>,
}

impl AllowAllScreening {
    /// Create a new AllowAllScreening with the given denylist
    pub fn new(denylist: Vec<String>) -> Self {
        Self { denylist }
    }

    /// Create a new AllowAllScreening from a comma-separated denylist string
    pub fn from_env(denylist_env: Option<String>) -> Self {
        let denylist = denylist_env
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self { denylist }
    }
}

#[async_trait]
impl Screening for AllowAllScreening {
    async fn check_country(&self, country_code: &str) -> ScreeningResult<()> {
        let upper_code = country_code.to_uppercase();
        if self.denylist.contains(&upper_code) {
            Err(ScreeningError::CountryBlocked(upper_code))
        } else {
            Ok(())
        }
    }

    fn name(&self) -> &'static str {
        "AllowAllScreening"
    }
}

/// No-op screening that always allows (used when flag is off)
#[derive(Debug, Clone)]
pub struct NoOpScreening;

#[async_trait]
impl Screening for NoOpScreening {
    async fn check_country(&self, _country_code: &str) -> ScreeningResult<()> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "NoOpScreening"
    }
}

/// Create the default screening implementation based on configuration
pub fn create_screening() -> Arc<dyn Screening> {
    if crate::card::is_card_enabled() {
        let denylist = std::env::var("CARD_COUNTRY_DENYLIST")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Arc::new(AllowAllScreening::new(denylist))
    } else {
        Arc::new(NoOpScreening)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allow_all_with_empty_denylist_allows_all() {
        let screening = AllowAllScreening::new(vec![]);

        assert!(screening.check_country("US").await.is_ok());
        assert!(screening.check_country("GB").await.is_ok());
        assert!(screening.check_country("DE").await.is_ok());
        assert!(screening.check_country("ng").await.is_ok()); // case insensitive
    }

    #[tokio::test]
    async fn test_allow_all_with_denylist_blocks_denied() {
        let screening = AllowAllScreening::new(vec!["NG".to_string(), "IR".to_string()]);

        assert!(screening.check_country("US").await.is_ok());
        assert!(screening.check_country("GB").await.is_ok());

        let result = screening.check_country("NG").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ScreeningError::CountryBlocked("NG".to_string())
        );

        let result = screening.check_country("ng").await; // case insensitive
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ScreeningError::CountryBlocked("NG".to_string())
        );

        let result = screening.check_country("IR").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ScreeningError::CountryBlocked("IR".to_string())
        );
    }

    #[tokio::test]
    async fn test_noop_screening_always_allows() {
        let screening = NoOpScreening;

        assert!(screening.check_country("US").await.is_ok());
        assert!(screening.check_country("NG").await.is_ok());
        assert!(screening.check_country("IR").await.is_ok());
        assert!(screening.check_country("XX").await.is_ok());
    }

    #[tokio::test]
    async fn test_create_screening_respects_flag() {
        std::env::remove_var("CARD_ENABLED");
        std::env::remove_var("CARD_COUNTRY_DENYLIST");

        let screening = create_screening();
        assert_eq!(screening.name(), "NoOpScreening");

        std::env::set_var("CARD_ENABLED", "true");
        let screening = create_screening();
        assert_eq!(screening.name(), "AllowAllScreening");

        std::env::set_var("CARD_COUNTRY_DENYLIST", "NG,IR");
        let screening = create_screening();
        assert_eq!(screening.name(), "AllowAllScreening");
        assert!(screening.check_country("NG").await.is_err());
        assert!(screening.check_country("US").await.is_ok());

        std::env::remove_var("CARD_ENABLED");
        std::env::remove_var("CARD_COUNTRY_DENYLIST");
    }

    #[test]
    fn test_from_env_parses_comma_separated() {
        let screening = AllowAllScreening::from_env(Some("NG, IR,  KP  ".to_string()));
        assert_eq!(screening.denylist, vec!["NG", "IR", "KP"]);
    }

    #[test]
    fn test_from_env_handles_empty() {
        let screening = AllowAllScreening::from_env(Some("".to_string()));
        assert!(screening.denylist.is_empty());

        let screening = AllowAllScreening::from_env(None);
        assert!(screening.denylist.is_empty());
    }
}
