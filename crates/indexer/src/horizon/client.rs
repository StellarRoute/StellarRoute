use crate::error::{IndexerError, Result};
use crate::models::horizon::{HorizonOffer, HorizonPage};
use std::time::Duration;
use tracing::{debug, warn};

/// Retry configuration for API requests
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Clone)]
pub struct HorizonClient {
    base_url: String,
    http: reqwest::Client,
    retry_config: RetryConfig,
}

/// Parameters for fetching an orderbook snapshot.
#[derive(Debug, Clone)]
pub struct OrderbookRequest<'a> {
    pub selling_asset_type: &'a str,
    pub selling_asset_code: Option<&'a str>,
    pub selling_asset_issuer: Option<&'a str>,
    pub buying_asset_type: &'a str,
    pub buying_asset_code: Option<&'a str>,
    pub buying_asset_issuer: Option<&'a str>,
    pub limit: Option<u32>,
}

impl HorizonClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_retry_config(base_url, RetryConfig::default())
    }

    pub fn with_retry_config(base_url: impl Into<String>, retry_config: RetryConfig) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            retry_config,
        }
    }

    /// Execute a request with exponential backoff retry logic
    async fn retry_request<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.retry_config.initial_delay_ms;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;

                    if !e.is_retryable() || attempt >= self.retry_config.max_retries {
                        match e.log_level() {
                            tracing::Level::ERROR => {
                                tracing::error!("Request failed after {} attempts: {}", attempt, e)
                            }
                            tracing::Level::WARN => {
                                tracing::warn!("Request failed after {} attempts: {}", attempt, e)
                            }
                            _ => tracing::info!("Request failed after {} attempts: {}", attempt, e),
                        }
                        return Err(e);
                    }

                    debug!(
                        "Request failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt, self.retry_config.max_retries, delay_ms, e
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                    delay_ms = ((delay_ms as f64) * self.retry_config.backoff_multiplier) as u64;
                    delay_ms = delay_ms.min(self.retry_config.max_delay_ms);
                }
            }
        }
    }

    /// Fetch offers page with retry logic.
    ///
    /// Confirmed endpoint: `GET /offers`
    /// Parameters:
    /// - `limit`: Number of offers to fetch (default: 200)
    /// - `cursor`: Pagination cursor (optional)
    /// - `selling`: Filter by selling asset (optional)
    /// - `buying`: Filter by buying asset (optional)
    pub async fn get_offers(
        &self,
        limit: Option<u32>,
        cursor: Option<&str>,
        selling: Option<&str>,
    ) -> Result<Vec<HorizonOffer>> {
        let limit = limit.unwrap_or(200);
        let mut url = format!("{}/offers?limit={}", self.base_url, limit);

        if let Some(c) = cursor {
            url.push_str("&cursor=");
            url.push_str(c);
        }

        if let Some(s) = selling {
            url.push_str("&selling=");
            url.push_str(s);
        }

        let client = self.http.clone();
        let url_clone = url.clone();

        self.retry_request(|| async {
            debug!("Fetching offers from: {}", url_clone);
            let resp = client.get(&url_clone).send().await?;

            let status = resp.status();
            if !status.is_success() {
                let error_body = resp.text().await.unwrap_or_default();
                return Err(IndexerError::StellarApi {
                    endpoint: url_clone.clone(),
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let page: HorizonPage<HorizonOffer> = resp.json().await?;
            Ok(page.embedded.records)
        })
        .await
    }

    /// Fetch orderbook snapshot for a trading pair.
    ///
    /// Endpoint: `GET /order_book`
    pub async fn get_orderbook(&self, req: OrderbookRequest<'_>) -> Result<serde_json::Value> {
        let limit = req.limit.unwrap_or(20);
        let mut url = format!(
            "{}/order_book?selling_asset_type={}&buying_asset_type={}&limit={}",
            self.base_url, req.selling_asset_type, req.buying_asset_type, limit
        );

        // Add optional parameters for selling asset
        if let Some(code) = req.selling_asset_code {
            url.push_str("&selling_asset_code=");
            url.push_str(code);
        }
        if let Some(issuer) = req.selling_asset_issuer {
            url.push_str("&selling_asset_issuer=");
            url.push_str(issuer);
        }

        // Add optional parameters for buying asset
        if let Some(code) = req.buying_asset_code {
            url.push_str("&buying_asset_code=");
            url.push_str(code);
        }
        if let Some(issuer) = req.buying_asset_issuer {
            url.push_str("&buying_asset_issuer=");
            url.push_str(issuer);
        }

        let client = self.http.clone();
        let url_clone = url.clone();

        self.retry_request(|| async {
            debug!("Fetching orderbook from: {}", url_clone);
            let resp = client.get(&url_clone).send().await?;

            let status = resp.status();
            if !status.is_success() {
                let error_body = resp.text().await.unwrap_or_default();
                return Err(IndexerError::StellarApi {
                    endpoint: url_clone.clone(),
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let orderbook: serde_json::Value = resp.json().await?;
            Ok(orderbook)
        })
        .await
    }

    /// Stream offers in real-time using Server-Sent Events (SSE).
    ///
    /// Endpoint: `GET /offers?cursor=now`
    /// This returns a stream that sends new offers as they are created.
    ///
    /// Note: This function returns an async stream that yields offers as they arrive.
    /// For now, we return a simple implementation that can be enhanced later.
    pub async fn stream_offers(&self) -> Result<impl futures::Stream<Item = Result<HorizonOffer>>> {
        use futures::stream::{self, StreamExt};

        let url = format!("{}/offers?cursor=now", self.base_url);
        debug!("Starting offer stream from: {}", url);

        // For now, return a polling-based stream
        // In production, this should use SSE (eventsource) for true streaming
        let client = self.clone();
        let stream = stream::unfold(None, move |cursor: Option<String>| {
            let client = client.clone();
            async move {
                // Poll for new offers
                match client.get_offers(Some(10), cursor.as_deref(), None).await {
                    Ok(offers) => {
                        if offers.is_empty() {
                            // No new offers, wait before next poll
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            Some((vec![], cursor))
                        } else {
                            // Return offers and update cursor
                            // In real Horizon API, cursor comes from paging info
                            Some((offers, Some("next_cursor".to_string())))
                        }
                    }
                    Err(e) => {
                        warn!("Error streaming offers: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        Some((vec![], cursor))
                    }
                }
            }
        })
        .flat_map(|offers| stream::iter(offers.into_iter().map(Ok)));

        Ok(stream)
    }

    /// Convert the Horizon asset JSON into our typed `Asset`.
    pub fn parse_asset(&self, v: &serde_json::Value) -> Result<crate::models::asset::Asset> {
        let asset_type = v
            .get("asset_type")
            .and_then(|x| x.as_str())
            .ok_or_else(|| IndexerError::MissingField {
                field: "asset_type".to_string(),
                context: "Horizon API asset response".to_string(),
            })?;

        match asset_type {
            "native" => Ok(crate::models::asset::Asset::Native),
            "credit_alphanum4" => Ok(crate::models::asset::Asset::CreditAlphanum4 {
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
            "credit_alphanum12" => Ok(crate::models::asset::Asset::CreditAlphanum12 {
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
                reason:
                    "Unknown asset type, expected: native, credit_alphanum4, or credit_alphanum12"
                        .to_string(),
            }),
        }
    }
}
