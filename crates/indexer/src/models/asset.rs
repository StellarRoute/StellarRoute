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
    fn test_asset_serialization() {
        let asset = Asset::Native;
        let json = serde_json::to_string(&asset).unwrap();
        assert!(json.contains("native"));
    }
}
