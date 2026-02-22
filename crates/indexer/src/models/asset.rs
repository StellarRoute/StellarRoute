use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "asset_type")]
pub enum Asset {
    #[serde(rename = "native")]
    Native,

    #[serde(rename = "credit_alphanum4")]
    CreditAlphanum4 {
        asset_code: String,
        asset_issuer: String,
    },

    #[serde(rename = "credit_alphanum12")]
    CreditAlphanum12 {
        asset_code: String,
        asset_issuer: String,
    },
}

impl Asset {
    pub fn key(&self) -> (String, Option<String>, Option<String>) {
        match self {
            Asset::Native => ("native".to_string(), None, None),
            Asset::CreditAlphanum4 {
                asset_code,
                asset_issuer,
            } => (
                "credit_alphanum4".to_string(),
                Some(asset_code.clone()),
                Some(asset_issuer.clone()),
            ),
            Asset::CreditAlphanum12 {
                asset_code,
                asset_issuer,
            } => (
                "credit_alphanum12".to_string(),
                Some(asset_code.clone()),
                Some(asset_issuer.clone()),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Asset::key()
    // -----------------------------------------------------------------------

    #[test]
    fn test_asset_native_key() {
        let asset = Asset::Native;
        let (asset_type, code, issuer) = asset.key();
        assert_eq!(asset_type, "native");
        assert_eq!(code, None);
        assert_eq!(issuer, None);
    }

    #[test]
    fn test_asset_credit_alphanum4_key() {
        let asset = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        };
        let (asset_type, code, issuer) = asset.key();
        assert_eq!(asset_type, "credit_alphanum4");
        assert_eq!(code, Some("USDC".to_string()));
        assert_eq!(
            issuer,
            Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
        );
    }

    #[test]
    fn test_asset_credit_alphanum12_key() {
        let asset = Asset::CreditAlphanum12 {
            asset_code: "YIELDXLM00".to_string(),
            asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        };
        let (asset_type, code, issuer) = asset.key();
        assert_eq!(asset_type, "credit_alphanum12");
        assert_eq!(code, Some("YIELDXLM00".to_string()));
        assert!(issuer.is_some(), "CreditAlphanum12 issuer should be Some");
    }

    // -----------------------------------------------------------------------
    // Equality
    // -----------------------------------------------------------------------

    #[test]
    fn test_native_equals_native() {
        assert_eq!(Asset::Native, Asset::Native);
    }

    #[test]
    fn test_native_not_equal_to_credit() {
        let credit = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        assert_ne!(Asset::Native, credit);
    }

    #[test]
    fn test_same_credit_assets_are_equal() {
        let a = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        let b = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_codes_not_equal() {
        let a = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        let b = Asset::CreditAlphanum4 {
            asset_code: "EURT".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_different_issuers_not_equal() {
        let a = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER1".to_string(),
        };
        let b = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER2".to_string(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_alphanum4_not_equal_to_alphanum12_same_code() {
        let a = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        let b = Asset::CreditAlphanum12 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // Serialization round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn test_asset_native_serializes_to_json_with_asset_type() {
        let asset = Asset::Native;
        let json = serde_json::to_string(&asset).unwrap();
        assert!(json.contains("native"));
    }

    #[test]
    fn test_asset_credit_alphanum4_serialization_round_trip() {
        let asset = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        };
        let json = serde_json::to_string(&asset).unwrap();
        let decoded: Asset = serde_json::from_str(&json).unwrap();
        assert_eq!(asset, decoded);
    }

    #[test]
    fn test_asset_credit_alphanum12_serialization_round_trip() {
        let asset = Asset::CreditAlphanum12 {
            asset_code: "YIELDXLM00".to_string(),
            asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        };
        let json = serde_json::to_string(&asset).unwrap();
        let decoded: Asset = serde_json::from_str(&json).unwrap();
        assert_eq!(asset, decoded);
    }

    #[test]
    fn test_asset_native_deserialization() {
        let json = r#"{"asset_type":"native"}"#;
        let asset: Asset = serde_json::from_str(json).unwrap();
        assert_eq!(asset, Asset::Native);
    }

    // -----------------------------------------------------------------------
    // Clone
    // -----------------------------------------------------------------------

    #[test]
    fn test_asset_clone_native() {
        let a = Asset::Native;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_asset_clone_credit() {
        let a = Asset::CreditAlphanum4 {
            asset_code: "USDC".to_string(),
            asset_issuer: "GISSUER".to_string(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
