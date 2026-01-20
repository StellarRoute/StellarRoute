//! Offer model for SDEX offers

use chrono::{DateTime, Utc};

use super::{asset::Asset, horizon::HorizonOffer};
use crate::error::{IndexerError, Result};

/// Normalized offer from SDEX
#[derive(Debug, Clone)]
pub struct Offer {
    pub id: u64,
    pub seller: String,
    pub selling: Asset,
    pub buying: Asset,
    pub amount: String,
    pub price_n: i32,
    pub price_d: i32,
    pub price: String,
    pub last_modified_ledger: u64,
    pub last_modified_time: Option<DateTime<Utc>>,
}

impl Offer {
    /// Validate offer data
    pub fn validate(&self) -> Result<()> {
        // Validate seller address (basic check - should be 56 chars starting with G)
        if !self.seller.starts_with('G') || self.seller.len() != 56 {
            return Err(IndexerError::StellarApi(format!(
                "Invalid seller address: {}",
                self.seller
            )));
        }

        // Validate amount is positive
        let amount_f64: f64 = self
            .amount
            .parse()
            .map_err(|_| IndexerError::StellarApi(format!("Invalid amount: {}", self.amount)))?;
        if amount_f64 <= 0.0 {
            return Err(IndexerError::StellarApi(format!(
                "Amount must be positive: {}",
                self.amount
            )));
        }

        // Validate price is positive
        let price_f64: f64 = self
            .price
            .parse()
            .map_err(|_| IndexerError::StellarApi(format!("Invalid price: {}", self.price)))?;
        if price_f64 <= 0.0 {
            return Err(IndexerError::StellarApi(format!(
                "Price must be positive: {}",
                self.price
            )));
        }

        // Validate price ratio
        if self.price_d == 0 {
            return Err(IndexerError::StellarApi(
                "Price denominator cannot be zero".to_string(),
            ));
        }

        // Validate that selling and buying assets are different
        if self.selling == self.buying {
            return Err(IndexerError::StellarApi(
                "Selling and buying assets must be different".to_string(),
            ));
        }

        Ok(())
    }
}

impl TryFrom<HorizonOffer> for Offer {
    type Error = IndexerError;

    fn try_from(horizon_offer: HorizonOffer) -> Result<Self> {
        let id = horizon_offer.id.parse::<u64>().map_err(|_| {
            IndexerError::StellarApi(format!("Invalid offer ID: {}", horizon_offer.id))
        })?;

        // Parse assets using the client's parse_asset method
        // We'll need to pass the client or make parse_asset a standalone function
        // For now, let's create a helper function
        let selling = parse_asset_from_value(&horizon_offer.selling)?;
        let buying = parse_asset_from_value(&horizon_offer.buying)?;

        let price_n = horizon_offer
            .price_r
            .as_ref()
            .map(|r| r.n as i32)
            .unwrap_or(0);
        let price_d = horizon_offer
            .price_r
            .as_ref()
            .map(|r| r.d as i32)
            .unwrap_or(1);

        let offer = Offer {
            id,
            seller: horizon_offer.seller,
            selling,
            buying,
            amount: horizon_offer.amount,
            price_n,
            price_d,
            price: horizon_offer.price,
            last_modified_ledger: horizon_offer.last_modified_ledger as u64,
            last_modified_time: None, // Horizon doesn't provide this directly
        };

        // Validate the offer before returning
        offer.validate()?;
        Ok(offer)
    }
}

fn parse_asset_from_value(v: &serde_json::Value) -> Result<Asset> {
    let asset_type = v
        .get("asset_type")
        .and_then(|x| x.as_str())
        .ok_or_else(|| IndexerError::StellarApi("missing asset_type".to_string()))?;

    match asset_type {
        "native" => Ok(Asset::Native),
        "credit_alphanum4" => Ok(Asset::CreditAlphanum4 {
            asset_code: v
                .get("asset_code")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::StellarApi("missing asset_code".to_string()))?
                .to_string(),
            asset_issuer: v
                .get("asset_issuer")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::StellarApi("missing asset_issuer".to_string()))?
                .to_string(),
        }),
        "credit_alphanum12" => Ok(Asset::CreditAlphanum12 {
            asset_code: v
                .get("asset_code")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::StellarApi("missing asset_code".to_string()))?
                .to_string(),
            asset_issuer: v
                .get("asset_issuer")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::StellarApi("missing asset_issuer".to_string()))?
                .to_string(),
        }),
        other => Err(IndexerError::StellarApi(format!(
            "unknown asset_type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::horizon::{HorizonOffer, HorizonPriceR};
    use serde_json::json;

    fn create_test_horizon_offer() -> HorizonOffer {
        HorizonOffer {
            id: "12345".to_string(),
            paging_token: Some("token123".to_string()),
            seller: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
            selling: json!({
                "asset_type": "native"
            }),
            buying: json!({
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            }),
            amount: "100.0".to_string(),
            price: "1.5".to_string(),
            price_r: Some(HorizonPriceR { n: 3, d: 2 }),
            last_modified_ledger: 12345,
        }
    }

    #[test]
    fn test_offer_from_horizon_offer() {
        let horizon_offer = create_test_horizon_offer();
        let offer = Offer::try_from(horizon_offer).unwrap();

        assert_eq!(offer.id, 12345);
        assert_eq!(offer.amount, "100.0");
        assert_eq!(offer.price, "1.5");
        assert_eq!(offer.price_n, 3);
        assert_eq!(offer.price_d, 2);
        assert_eq!(offer.last_modified_ledger, 12345);
        assert!(matches!(offer.selling, Asset::Native));
        assert!(matches!(offer.buying, Asset::CreditAlphanum4 { .. }));
    }

    #[test]
    fn test_offer_invalid_id() {
        let mut horizon_offer = create_test_horizon_offer();
        horizon_offer.id = "invalid".to_string();

        let result = Offer::try_from(horizon_offer);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_asset_native() {
        let json = json!({"asset_type": "native"});
        let asset = parse_asset_from_value(&json).unwrap();
        assert!(matches!(asset, Asset::Native));
    }

    #[test]
    fn test_parse_asset_credit_alphanum4() {
        let json = json!({
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        });
        let asset = parse_asset_from_value(&json).unwrap();
        match asset {
            Asset::CreditAlphanum4 {
                asset_code,
                asset_issuer,
            } => {
                assert_eq!(asset_code, "USDC");
                assert_eq!(
                    asset_issuer,
                    "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                );
            }
            _ => panic!("Expected CreditAlphanum4"),
        }
    }
}
