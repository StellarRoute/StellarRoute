use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "asset_type")]
pub enum Asset {
    #[serde(rename = "native")]
    Native,

    #[serde(rename = "credit_alphanum4")]
    CreditAlphanum4 { asset_code: String, asset_issuer: String },

    #[serde(rename = "credit_alphanum12")]
    CreditAlphanum12 { asset_code: String, asset_issuer: String },
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

