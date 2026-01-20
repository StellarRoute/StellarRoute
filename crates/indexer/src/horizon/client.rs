use crate::error::{IndexerError, Result};
use crate::models::horizon::{HorizonOffer, HorizonPage};

#[derive(Clone)]
pub struct HorizonClient {
    base_url: String,
    http: reqwest::Client,
}

impl HorizonClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Fetch offers page.
    ///
    /// Confirmed endpoint: `GET /offers`
    pub async fn get_offers(&self, limit: u32, cursor: Option<&str>) -> Result<HorizonPage<HorizonOffer>> {
        let mut url = format!("{}/offers?limit={}", self.base_url, limit);
        if let Some(c) = cursor {
            url.push_str("&cursor=");
            url.push_str(c);
        }

        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json::<HorizonPage<HorizonOffer>>().await?)
    }

    /// Convert the Horizon asset JSON into our typed `Asset`.
    pub fn parse_asset(&self, v: &serde_json::Value) -> Result<crate::models::asset::Asset> {
        // Horizon uses objects like:
        // { "asset_type":"native" } OR
        // { "asset_type":"credit_alphanum4","asset_code":"USDC","asset_issuer":"G..." }
        let asset_type = v
            .get("asset_type")
            .and_then(|x| x.as_str())
            .ok_or_else(|| IndexerError::StellarApi("missing asset_type".to_string()))?;

        match asset_type {
            "native" => Ok(crate::models::asset::Asset::Native),
            "credit_alphanum4" => Ok(crate::models::asset::Asset::CreditAlphanum4 {
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
            "credit_alphanum12" => Ok(crate::models::asset::Asset::CreditAlphanum12 {
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
            other => Err(IndexerError::StellarApi(format!("unknown asset_type: {other}"))),
        }
    }
}

