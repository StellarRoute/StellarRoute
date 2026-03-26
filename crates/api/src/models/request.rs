//! API request models

use serde::Deserialize;

/// Default slippage tolerance in basis points (0.50%)
pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;
/// Maximum slippage tolerance in basis points (100.00%)
pub const MAX_SLIPPAGE_BPS: u32 = 10_000;

/// Query parameters for quote endpoint
#[derive(Debug, Deserialize)]
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

impl QuoteParams {
    /// Get the slippage tolerance in basis points, applying default if omitted
    pub fn slippage_bps(&self) -> u32 {
        self.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS)
    }

    /// Validate the slippage tolerance bounds
    pub fn validate_slippage(&self) -> std::result::Result<(), String> {
        let bps = self.slippage_bps();
        if bps > MAX_SLIPPAGE_BPS {
            return Err(format!(
                "slippage_bps must be between 0 and {} (100%)",
                MAX_SLIPPAGE_BPS
            ));
        }
        Ok(())
    }
}

fn default_quote_type() -> QuoteType {
    QuoteType::Sell
}

/// Type of quote requested
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum QuoteType {
    /// Selling the base asset
    Sell,
    /// Buying the base asset
    Buy,
}

/// Asset identifier in path parameters
#[derive(Debug, Deserialize)]
pub struct AssetPath {
    /// Asset code (e.g., "XLM", "USDC", or "native" for XLM)
    pub asset_code: String,
    /// Asset issuer (optional, only for issued assets)
    pub asset_issuer: Option<String>,
}

impl AssetPath {
    /// Parse asset identifier from path segment
    /// Format: "native" or "CODE" or "CODE:ISSUER"
    pub fn parse(s: &str) -> Result<Self, String> {
        if s == "native" {
            return Ok(Self {
                asset_code: "native".to_string(),
                asset_issuer: None,
            });
        }

        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => Ok(Self {
                asset_code: parts[0].to_uppercase(),
                asset_issuer: None,
            }),
            2 => Ok(Self {
                asset_code: parts[0].to_uppercase(),
                asset_issuer: Some(parts[1].to_string()),
            }),
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
        };
        assert_eq!(params.slippage_bps(), DEFAULT_SLIPPAGE_BPS);
        assert!(params.validate_slippage().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_valid() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(100),
            quote_type: QuoteType::Sell,
        };
        assert_eq!(params.slippage_bps(), 100);
        assert!(params.validate_slippage().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_boundary_max() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(MAX_SLIPPAGE_BPS),
            quote_type: QuoteType::Sell,
        };
        assert_eq!(params.slippage_bps(), MAX_SLIPPAGE_BPS);
        assert!(params.validate_slippage().is_ok());
    }

    #[test]
    fn test_quote_params_slippage_invalid_too_high() {
        let params = QuoteParams {
            amount: None,
            slippage_bps: Some(MAX_SLIPPAGE_BPS + 1),
            quote_type: QuoteType::Sell,
        };
        let result = params.validate_slippage();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            format!(
                "slippage_bps must be between 0 and {} (100%)",
                MAX_SLIPPAGE_BPS
            )
        );
    }
}
