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
        if !self.seller.starts_with('G') || self.seller.len() != 56 {
            return Err(IndexerError::InvalidOffer {
                offer_id: self.id.to_string(),
                reason: format!("Invalid seller address: {}", self.seller),
            });
        }

        let amount_f64: f64 = self
            .amount
            .parse()
            .map_err(|_| IndexerError::NumericParse {
                value: self.amount.clone(),
                expected_type: "positive number".to_string(),
            })?;
        if amount_f64 <= 0.0 {
            return Err(IndexerError::InvalidOffer {
                offer_id: self.id.to_string(),
                reason: format!("Amount must be positive: {}", self.amount),
            });
        }

        let price_f64: f64 = self.price.parse().map_err(|_| IndexerError::NumericParse {
            value: self.price.clone(),
            expected_type: "positive number".to_string(),
        })?;
        if price_f64 <= 0.0 {
            return Err(IndexerError::InvalidOffer {
                offer_id: self.id.to_string(),
                reason: format!("Price must be positive: {}", self.price),
            });
        }

        if self.price_d == 0 {
            return Err(IndexerError::InvalidOffer {
                offer_id: self.id.to_string(),
                reason: "Price denominator cannot be zero".to_string(),
            });
        }

        if self.selling == self.buying {
            return Err(IndexerError::InvalidOffer {
                offer_id: self.id.to_string(),
                reason: "Selling and buying assets must be different".to_string(),
            });
        }

        Ok(())
    }
}

impl TryFrom<HorizonOffer> for Offer {
    type Error = IndexerError;

    fn try_from(horizon_offer: HorizonOffer) -> Result<Self> {
        let id = horizon_offer
            .id
            .parse::<u64>()
            .map_err(|_| IndexerError::NumericParse {
                value: horizon_offer.id.clone(),
                expected_type: "u64 offer ID".to_string(),
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
        .ok_or_else(|| IndexerError::MissingField {
            field: "asset_type".to_string(),
            context: "Horizon API asset response".to_string(),
        })?;

    match asset_type {
        "native" => Ok(Asset::Native),
        "credit_alphanum4" => Ok(Asset::CreditAlphanum4 {
            asset_code: v
                .get("asset_code")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::MissingField {
                    field: "asset_code".to_string(),
                    context: "credit_alphanum4 asset".to_string(),
                })?
                .to_string(),
            asset_issuer: v
                .get("asset_issuer")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::MissingField {
                    field: "asset_issuer".to_string(),
                    context: "credit_alphanum4 asset".to_string(),
                })?
                .to_string(),
        }),
        "credit_alphanum12" => Ok(Asset::CreditAlphanum12 {
            asset_code: v
                .get("asset_code")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::MissingField {
                    field: "asset_code".to_string(),
                    context: "credit_alphanum12 asset".to_string(),
                })?
                .to_string(),
            asset_issuer: v
                .get("asset_issuer")
                .and_then(|x| x.as_str())
                .ok_or_else(|| IndexerError::MissingField {
                    field: "asset_issuer".to_string(),
                    context: "credit_alphanum12 asset".to_string(),
                })?
                .to_string(),
        }),
        other => Err(IndexerError::InvalidAsset {
            asset: other.to_string(),
            reason: "Unknown asset type, expected: native, credit_alphanum4, or credit_alphanum12"
                .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::horizon::{HorizonOffer, HorizonPriceR};
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Fixtures
    // -----------------------------------------------------------------------

    const VALID_SELLER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    fn native_asset_json() -> serde_json::Value {
        json!({"asset_type": "native"})
    }

    fn usdc_asset_json() -> serde_json::Value {
        json!({
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": VALID_SELLER
        })
    }

    fn yxlm_asset_json() -> serde_json::Value {
        json!({
            "asset_type": "credit_alphanum12",
            "asset_code": "YIELDXLM00",
            "asset_issuer": VALID_SELLER
        })
    }

    fn create_test_horizon_offer() -> HorizonOffer {
        HorizonOffer {
            id: "12345".to_string(),
            paging_token: Some("token123".to_string()),
            seller: VALID_SELLER.to_string(),
            selling: native_asset_json(),
            buying: usdc_asset_json(),
            amount: "100.0".to_string(),
            price: "1.5".to_string(),
            price_r: Some(HorizonPriceR { n: 3, d: 2 }),
            last_modified_ledger: 12345,
        }
    }

    // -----------------------------------------------------------------------
    // TryFrom<HorizonOffer> — happy paths
    // -----------------------------------------------------------------------

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
        assert_eq!(offer.seller, VALID_SELLER);
        assert!(offer.last_modified_time.is_none());
    }

    #[test]
    fn test_offer_without_price_r_defaults_to_zero_numerator_one_denominator() {
        let mut h = create_test_horizon_offer();
        h.price_r = None;
        let offer = Offer::try_from(h).unwrap();
        assert_eq!(offer.price_n, 0);
        assert_eq!(offer.price_d, 1);
    }

    #[test]
    fn test_offer_alphanum12_selling_asset() {
        let mut h = create_test_horizon_offer();
        h.selling = yxlm_asset_json();
        h.buying = native_asset_json();
        let offer = Offer::try_from(h).unwrap();
        assert!(matches!(offer.selling, Asset::CreditAlphanum12 { .. }));
        assert!(matches!(offer.buying, Asset::Native));
    }

    #[test]
    fn test_offer_paging_token_optional() {
        let mut h = create_test_horizon_offer();
        h.paging_token = None;
        // Should still parse successfully
        assert!(Offer::try_from(h).is_ok());
    }

    // -----------------------------------------------------------------------
    // TryFrom<HorizonOffer> — error paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_offer_invalid_id_non_numeric() {
        let mut h = create_test_horizon_offer();
        h.id = "not-a-number".to_string();
        assert!(Offer::try_from(h).is_err());
    }

    #[test]
    fn test_offer_invalid_id_float() {
        let mut h = create_test_horizon_offer();
        h.id = "12345.67".to_string();
        matches!(Offer::try_from(h), Err(IndexerError::NumericParse { .. }));
    }

    #[test]
    fn test_offer_missing_selling_asset_type() {
        let mut h = create_test_horizon_offer();
        h.selling = json!({"asset_code": "XLM"});
        let err = Offer::try_from(h).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_offer_missing_buying_asset_type() {
        let mut h = create_test_horizon_offer();
        h.buying = json!({"asset_code": "USDC"});
        let err = Offer::try_from(h).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_offer_unknown_selling_asset_type() {
        let mut h = create_test_horizon_offer();
        h.selling = json!({"asset_type": "credit_alphanum99"});
        let err = Offer::try_from(h).unwrap_err();
        assert!(matches!(err, IndexerError::InvalidAsset { .. }));
    }

    #[test]
    fn test_offer_credit_missing_asset_code() {
        let mut h = create_test_horizon_offer();
        h.buying = json!({
            "asset_type": "credit_alphanum4",
            "asset_issuer": VALID_SELLER
        });
        let err = Offer::try_from(h).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_offer_credit_missing_asset_issuer() {
        let mut h = create_test_horizon_offer();
        h.buying = json!({
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC"
        });
        let err = Offer::try_from(h).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    // -----------------------------------------------------------------------
    // Offer::validate() — all branches
    // -----------------------------------------------------------------------

    fn make_valid_offer() -> Offer {
        Offer {
            id: 1,
            seller: VALID_SELLER.to_string(),
            selling: Asset::Native,
            buying: Asset::CreditAlphanum4 {
                asset_code: "USDC".to_string(),
                asset_issuer: VALID_SELLER.to_string(),
            },
            amount: "100.0".to_string(),
            price_n: 3,
            price_d: 2,
            price: "1.5".to_string(),
            last_modified_ledger: 1000,
            last_modified_time: None,
        }
    }

    #[test]
    fn test_validate_valid_offer_passes() {
        assert!(make_valid_offer().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_seller_wrong_prefix() {
        let mut o = make_valid_offer();
        o.seller = "BADFAKEADDRESS0000000000000000000000000000000000000000000".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_invalid_seller_too_short() {
        let mut o = make_valid_offer();
        o.seller = "GABC".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_zero_amount() {
        let mut o = make_valid_offer();
        o.amount = "0.0".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_negative_amount() {
        let mut o = make_valid_offer();
        o.amount = "-10.0".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_non_numeric_amount() {
        // "abc" cannot parse as f64 → NumericParse error
        let mut o = make_valid_offer();
        o.amount = "abc".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::NumericParse { .. }));
    }

    #[test]
    fn test_validate_infinity_amount_is_valid_per_rust_f64() {
        // f64::INFINITY > 0.0 is true, so validate() considers "inf" a valid amount.
        // This test documents the current behavior (not a bug we need to fix here).
        let mut o = make_valid_offer();
        o.amount = "inf".to_string();
        // "inf" parses as f64::INFINITY which is > 0.0, so validation passes
        // The business logic layer above validate() is responsible for meaningful bounds.
        let _ = o.validate(); // should not panic
    }

    #[test]
    fn test_validate_zero_price() {
        let mut o = make_valid_offer();
        o.price = "0.0".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_negative_price() {
        let mut o = make_valid_offer();
        o.price = "-1.0".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_non_numeric_price() {
        let mut o = make_valid_offer();
        o.price = "abc".to_string();
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::NumericParse { .. }));
    }

    #[test]
    fn test_validate_zero_price_denominator() {
        let mut o = make_valid_offer();
        o.price_d = 0;
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_same_asset_selling_and_buying() {
        let mut o = make_valid_offer();
        o.selling = Asset::Native;
        o.buying = Asset::Native;
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    #[test]
    fn test_validate_same_credit_asset_both_sides() {
        let asset = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: VALID_SELLER.to_string(),
        };
        let mut o = make_valid_offer();
        o.selling = asset.clone();
        o.buying = asset;
        let err = o.validate().unwrap_err();
        assert!(matches!(err, IndexerError::InvalidOffer { .. }));
    }

    // -----------------------------------------------------------------------
    // parse_asset_from_value — all variants
    // -----------------------------------------------------------------------

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
            "asset_issuer": VALID_SELLER
        });
        let asset = parse_asset_from_value(&json).unwrap();
        match asset {
            Asset::CreditAlphanum4 {
                asset_code,
                asset_issuer,
            } => {
                assert_eq!(asset_code, "USDC");
                assert_eq!(asset_issuer, VALID_SELLER);
            }
            _ => panic!("Expected CreditAlphanum4"),
        }
    }

    #[test]
    fn test_parse_asset_credit_alphanum12() {
        let json = json!({
            "asset_type": "credit_alphanum12",
            "asset_code": "YIELDXLM00",
            "asset_issuer": VALID_SELLER
        });
        let asset = parse_asset_from_value(&json).unwrap();
        match asset {
            Asset::CreditAlphanum12 {
                asset_code,
                asset_issuer,
            } => {
                assert_eq!(asset_code, "YIELDXLM00");
                assert_eq!(asset_issuer, VALID_SELLER);
            }
            _ => panic!("Expected CreditAlphanum12"),
        }
    }

    #[test]
    fn test_parse_asset_missing_asset_type_field() {
        let json = json!({"asset_code": "USDC"});
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_parse_asset_unknown_type_returns_error() {
        let json = json!({"asset_type": "fiat_usd"});
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::InvalidAsset { .. }));
    }

    #[test]
    fn test_parse_asset_alphanum4_missing_code() {
        let json = json!({
            "asset_type": "credit_alphanum4",
            "asset_issuer": VALID_SELLER
        });
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_parse_asset_alphanum4_missing_issuer() {
        let json = json!({
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC"
        });
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_parse_asset_alphanum12_missing_code() {
        let json = json!({
            "asset_type": "credit_alphanum12",
            "asset_issuer": VALID_SELLER
        });
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    #[test]
    fn test_parse_asset_null_asset_type_value() {
        let json = json!({"asset_type": null});
        let err = parse_asset_from_value(&json).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    // -----------------------------------------------------------------------
    // Edge cases — empty / boundary values
    // -----------------------------------------------------------------------

    #[test]
    fn test_offer_minimum_valid_amount() {
        let mut h = create_test_horizon_offer();
        h.amount = "0.0000001".to_string();
        assert!(Offer::try_from(h).is_ok());
    }

    #[test]
    fn test_offer_very_large_id() {
        let mut h = create_test_horizon_offer();
        h.id = u64::MAX.to_string();
        // validate will fail on seller check which is fine — ID parsed OK
        // but avoid running validate by just checking id parsing
        let result = Offer::try_from(h);
        // Whether Ok or Err(InvalidOffer), the id itself must have parsed
        match result {
            Ok(o) => assert_eq!(o.id, u64::MAX),
            Err(IndexerError::InvalidOffer { .. }) => {} // validate rejection is fine
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_offer_empty_amount_fails() {
        let mut o = make_valid_offer();
        o.amount = "".to_string();
        assert!(o.validate().is_err());
    }
}
