//! API request models

use serde::Deserialize;
use utoipa::ToSchema;

/// Default slippage tolerance in basis points (0.50%)
pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;
/// Maximum slippage tolerance in basis points (100.00%)
pub const MAX_SLIPPAGE_BPS: u32 = 10_000;

/// Query parameters for quote endpoint
#[derive(Debug, Deserialize, Clone)]
pub struct QuoteParams {
    /// Amount to trade
    pub amount: Option<String>,
    /// Slippage tolerance in basis points (e.g. 50 = 0.50%)
    pub slippage_bps: Option<u32>,
    /// Type of quote (buy or sell)
    #[serde(default = "default_quote_type")]
    pub quote_type: QuoteType,
    /// Explain the route selection with decision diagnostics
    pub explain: Option<bool>,
}

/// Request item for batch quotes
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct QuoteRequestItem {
    /// Base asset identifier ("native", "CODE", or "CODE:ISSUER")
    pub base: String,
    /// Quote asset identifier ("native", "CODE", or "CODE:ISSUER")
    pub quote: String,
    /// Amount to trade (default: "1")
    pub amount: Option<String>,
    /// Slippage tolerance in basis points (default: 50)
    pub slippage_bps: Option<u32>,
    /// Quote direction: "sell" or "buy" (default: sell)
    pub quote_type: Option<QuoteType>,
}

impl QuoteRequestItem {
    /// Validate this item and return a descriptive error string on failure.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.base.is_empty() {
            return Err("base asset cannot be empty".to_string());
        }
        if self.quote.is_empty() {
            return Err("quote asset cannot be empty".to_string());
        }
        if self.base == self.quote {
            return Err(format!(
                "base and quote assets must differ (got '{}')",
                self.base
            ));
        }
        if let Some(ref amount_str) = self.amount {
            let amount: f64 = amount_str
                .parse()
                .map_err(|_| format!("amount '{}' is not a valid number", amount_str))?;
            if amount <= 0.0 {
                return Err(format!("amount must be > 0, got {}", amount));
            }
        }
        if let Some(bps) = self.slippage_bps {
            if bps > MAX_SLIPPAGE_BPS {
                return Err(format!(
                    "slippage_bps {} exceeds maximum {}",
                    bps, MAX_SLIPPAGE_BPS
                ));
            }
        }
        Ok(())
    }
}

/// Batch quote request — up to 25 pairs in one call.
///
/// All items are executed concurrently against a shared market snapshot.
/// Per-item failures do not abort the batch; each item carries its own
/// `result` field indicating success or the specific error.
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct BatchQuoteRequest {
    /// Quote items to evaluate (1–25).
    pub quotes: Vec<QuoteRequestItem>,
}

/// Query parameters for the multiple-routes endpoint
#[derive(Debug, Deserialize)]
pub struct RoutesParams {
    pub amount: Option<String>,
    pub limit: Option<usize>,
    pub max_hops: Option<usize>,
    pub environment: Option<String>,
}

impl QuoteParams {
    /// Get the slippage tolerance in basis points, applying default if omitted
    pub fn slippage_bps(&self) -> u32 {
        self.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS)
    }

    /// Validate the parameters for common requirements
    pub fn validate(&self) -> std::result::Result<(), (String, String)> {
        if let Some(ref amount_str) = self.amount {
            let amount: f64 = amount_str.parse().map_err(|_| {
                (
                    "invalid_amount".to_string(),
                    "Amount must be a numeric string".to_string(),
                )
            })?;
            if amount <= 0.0 {
                return Err((
                    "invalid_amount".to_string(),
                    "Amount must be greater than zero".to_string(),
                ));
            }
        }

        if self.slippage_bps() > MAX_SLIPPAGE_BPS {
            return Err((
                "invalid_slippage".to_string(),
                format!(
                    "slippage_bps must be between 0 and {} (100%)",
                    MAX_SLIPPAGE_BPS
                ),
            ));
        }

        Ok(())
    }
}

fn default_quote_type() -> QuoteType {
    QuoteType::Sell
}

/// Type of quote requested
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum QuoteType {
    /// Selling the base asset
    Sell,
    /// Buying the base asset
    Buy,
}

/// Asset identifier in path parameters
#[derive(Debug, Clone, Deserialize)]
pub struct AssetPath {
    /// Asset code (e.g., "XLM", "USDC", or "native" for XLM)
    pub asset_code: String,
    /// Asset issuer (optional, only for issued assets)
    pub asset_issuer: Option<String>,
}

impl AssetPath {
    /// Parse asset identifier from path segment
    /// Format: "native" or "CODE" or "CODE:ISSUER"
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        if s == "native" {
            return Ok(Self {
                asset_code: "native".to_string(),
                asset_issuer: None,
            });
        }

        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                let code = parts[0].to_uppercase();
                if code.is_empty() {
                    return Err(format!("Asset code cannot be empty: {}", s));
                }
                Ok(Self {
                    asset_code: code,
                    asset_issuer: None,
                })
            }
            2 => {
                let code = parts[0].to_uppercase();
                let issuer = parts[1];
                if code.is_empty() || issuer.is_empty() {
                    return Err(format!("Asset code and issuer cannot be empty: {}", s));
                }
                Ok(Self {
                    asset_code: code,
                    asset_issuer: Some(issuer.to_string()),
                })
            }
            _ => Err(format!("Invalid asset format: {}", s)),
        }
    }

    /// Convert to asset type for database queries
    pub fn to_asset_type(&self) -> String {
        if self.asset_code == "native" {
            "native".to_string()
        } else {
            "credit_alphanum4".to_string() // Simplified, would need to detect alphanum12
        }
    }

    /// Canonical Stellar asset identifier: "native" or "CODE:ISSUER"
    pub fn to_canonical(&self) -> String {
        match (&self.asset_code.as_str(), &self.asset_issuer) {
            (&"native", _) => "native".to_string(),
            (code, Some(issuer)) => format!("{}:{}", code, issuer),
            (code, None) => code.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_native_asset() {
        let asset = AssetPath::parse("native").unwrap();
        assert_eq!(asset.asset_code, "native");
        assert_eq!(asset.asset_issuer, None);
    }

    #[test]
    fn test_parse_code_only() {
        let asset = AssetPath::parse("USDC").unwrap();
        assert_eq!(asset.asset_code, "USDC");
        assert_eq!(asset.asset_issuer, None);
    }

    #[test]
    fn test_parse_code_and_issuer() {
        let asset =
            AssetPath::parse("USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5")
                .unwrap();
        assert_eq!(asset.asset_code, "USDC");
        assert_eq!(
            asset.asset_issuer.as_deref(),
            Some("GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5")
        );
    }

    #[test]
    fn test_quote_params_slippage_default() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: None,
            quote_type: QuoteType::Sell,
            explain: None,
        };
        assert_eq!(params.slippage_bps(), DEFAULT_SLIPPAGE_BPS);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_valid() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(100),
            quote_type: QuoteType::Sell,
            explain: None,
        };
        assert_eq!(params.slippage_bps(), 100);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_boundary_max() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(MAX_SLIPPAGE_BPS),
            quote_type: QuoteType::Sell,
            explain: None,
        };
        assert_eq!(params.slippage_bps(), MAX_SLIPPAGE_BPS);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_invalid_too_high() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(MAX_SLIPPAGE_BPS + 1),
            quote_type: QuoteType::Sell,
            explain: None,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, "invalid_slippage");
    }

    #[test]
    fn test_quote_params_invalid_amount() {
        let params = QuoteParams {
            amount: Some("abc".to_string()),
            slippage_bps: None,
            quote_type: QuoteType::Sell,
            explain: None,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, "invalid_amount");
    }

    #[test]
    fn test_quote_params_zero_amount() {
        let params = QuoteParams {
            amount: Some("0".to_string()),
            slippage_bps: None,
            quote_type: QuoteType::Sell,
            explain: None,
        };
        let result = params.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, "invalid_amount");
    }
}
