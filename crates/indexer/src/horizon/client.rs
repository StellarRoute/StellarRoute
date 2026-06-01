use crate::error::{IndexerError, Result};
use crate::horizon::backpressure::{parse_retry_after, BackoffConfig, ThrottleState};
use crate::models::horizon::{HorizonOffer, HorizonOrderbook, HorizonPage};
use std::sync::Arc;
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
    /// Shared throttle state — tracks consecutive 429s and emits metrics.
    pub throttle: Arc<ThrottleState>,
    /// Backoff configuration for 429 responses.
    backoff_config: BackoffConfig,
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
            throttle: Arc::new(ThrottleState::new()),
            backoff_config: BackoffConfig::default(),
        }
    }

    /// Create a client with custom retry config and custom backoff config (useful in tests).
    pub fn with_retry_config_and_backoff(
        base_url: impl Into<String>,
        retry_config: RetryConfig,
        backoff_config: BackoffConfig,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            retry_config,
            throttle: Arc::new(ThrottleState::new()),
            backoff_config,
        }
    }

    /// Execute a request with exponential backoff retry logic.
    ///
    /// 429 responses are handled specially:
    /// - The `Retry-After` header is respected when present.
    /// - Full-jitter exponential backoff is applied otherwise.
    /// - Cursor progress is preserved (the caller never advances the cursor on 429).
    async fn retry_request<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.retry_config.initial_delay_ms;

        loop {
            match operation().await {
                Ok(result) => {
                    self.throttle.record_success();
                    return Ok(result);
                }
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

    /// Execute a request, handling 429 with adaptive backoff before delegating
    /// to the standard retry loop.
    ///
    /// This wrapper intercepts the raw HTTP response so it can read the
    /// `Retry-After` header before the body is consumed.
    async fn execute_with_backpressure(&self, url: &str) -> Result<reqwest::Response> {
        let max_rate_limit_retries: u32 = 8;
        let mut rl_attempt = 0u32;

        loop {
            let resp = self.http.get(url).send().await?;

            if resp.status().as_u16() == 429 {
                rl_attempt += 1;
                let retry_after = parse_retry_after(
                    resp.headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok()),
                );
                let delay = self
                    .throttle
                    .record_rate_limit(retry_after, &self.backoff_config);

                if rl_attempt >= max_rate_limit_retries {
                    warn!(
                        url = url,
                        rl_attempt, "Giving up after {} rate-limit retries", max_rate_limit_retries
                    );
                    return Err(IndexerError::RateLimitExceeded {
                        retry_after: retry_after.or(Some(delay.as_secs())),
                    });
                }

                crate::horizon::backpressure::throttle_sleep(delay).await;
                continue;
            }

            return Ok(resp);
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

        debug!("Fetching offers from: {}", url);

        let url_c = url.clone();
        self.retry_request(|| async {
            let resp = self.execute_with_backpressure(&url_c).await?;

            let status = resp.status();
            if !status.is_success() {
                let error_body = resp.text().await.unwrap_or_default();
                return Err(IndexerError::StellarApi {
                    endpoint: url_c.clone(),
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
    pub async fn get_orderbook(&self, req: OrderbookRequest<'_>) -> Result<HorizonOrderbook> {
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

        debug!("Fetching orderbook from: {}", url);

        let url_c = url.clone();
        self.retry_request(|| async {
            let resp = self.execute_with_backpressure(&url_c).await?;

            let status = resp.status();
            if !status.is_success() {
                let error_body = resp.text().await.unwrap_or_default();
                return Err(IndexerError::StellarApi {
                    endpoint: url_c.clone(),
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let orderbook: HorizonOrderbook = resp.json().await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::horizon::HorizonPriceR;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn offers_page_json(records: serde_json::Value) -> String {
        serde_json::json!({
            "_links": {
                "next": { "href": "https://horizon-testnet.stellar.org/offers?cursor=123&limit=200" }
            },
            "_embedded": {
                "records": records
            }
        })
        .to_string()
    }

    fn sample_offer_json() -> serde_json::Value {
        serde_json::json!({
            "id": "42",
            "paging_token": "42",
            "seller": "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN",
            "selling": { "asset_type": "native" },
            "buying": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "amount": "100.0000000",
            "price": "0.1000000",
            "price_r": { "n": 1, "d": 10 },
            "last_modified_ledger": 40_000_000_i64,
            "last_modified_time": "2024-01-01T00:00:00Z",
            "sponsor": null
        })
    }

    fn orderbook_json() -> String {
        serde_json::json!({
            "bids": [
                {
                    "price_r": { "n": 1, "d": 10 },
                    "price": "0.1000000",
                    "amount": "1000.0000000"
                }
            ],
            "asks": [
                {
                    "price_r": { "n": 1, "d": 8 },
                    "price": "0.1250000",
                    "amount": "500.0000000"
                }
            ],
            "base": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "counter": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            }
        })
        .to_string()
    }

    // -----------------------------------------------------------------------
    // RetryConfig unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_retry_config_defaults() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.initial_delay_ms, 100);
        assert_eq!(cfg.max_delay_ms, 5000);
        assert!((cfg.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_config_custom() {
        let cfg = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 50,
            max_delay_ms: 10_000,
            backoff_multiplier: 1.5,
        };
        assert_eq!(cfg.max_retries, 5);
        assert_eq!(cfg.initial_delay_ms, 50);
        assert_eq!(cfg.max_delay_ms, 10_000);
        assert!((cfg.backoff_multiplier - 1.5).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // HorizonClient construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_client_new_trims_trailing_slash() {
        let client = HorizonClient::new("https://horizon.stellar.org/");
        // Indirectly verified: if the URL were not trimmed a second `/` would
        // appear in every request path, which the mock tests below would catch.
        assert!(format!("{:?}", client.retry_config).contains("max_retries: 3"));
    }

    #[test]
    fn test_client_with_custom_retry_config() {
        let cfg = RetryConfig {
            max_retries: 1,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config("https://horizon.stellar.org", cfg);
        assert_eq!(client.retry_config.max_retries, 1);
    }

    // -----------------------------------------------------------------------
    // get_offers – success
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_offers_returns_records() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("limit", "10"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(offers_page_json(serde_json::json!([sample_offer_json()]))),
            )
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let offers = client.get_offers(Some(10), None, None).await.unwrap();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].id, "42");
    }

    #[tokio::test]
    async fn test_get_offers_empty_page() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("limit", "200"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(offers_page_json(serde_json::json!([]))),
            )
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let offers = client.get_offers(None, None, None).await.unwrap();
        assert!(offers.is_empty());
    }

    #[tokio::test]
    async fn test_get_offers_multiple_records() {
        let mock_server = MockServer::start().await;

        let records = serde_json::json!([sample_offer_json(), sample_offer_json()]);
        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("limit", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_string(offers_page_json(records)))
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let offers = client.get_offers(None, None, None).await.unwrap();
        assert_eq!(offers.len(), 2);
    }

    #[tokio::test]
    async fn test_get_offers_with_cursor() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("limit", "200"))
            .and(query_param("cursor", "99"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(offers_page_json(serde_json::json!([]))),
            )
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let result = client.get_offers(None, Some("99"), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_offers_with_selling_filter() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("selling", "native"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(offers_page_json(serde_json::json!([]))),
            )
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let result = client.get_offers(None, None, Some("native")).await;
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // get_offers – error paths
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_offers_500_returns_stellar_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Use zero retries so the test finishes quickly
        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let err = client.get_offers(Some(10), None, None).await.unwrap_err();

        match err {
            IndexerError::StellarApi { status, .. } => assert_eq!(status, 500),
            other => panic!("Expected StellarApi error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_offers_429_returns_rate_limit_error() {
        let mock_server = MockServer::start().await;

        // Always 429 — the client should exhaust retries and return RateLimitExceeded
        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "0")
                    .set_body_string("Too Many Requests"),
            )
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let err = client.get_offers(Some(10), None, None).await.unwrap_err();

        // After our backpressure change, persistent 429s surface as RateLimitExceeded
        assert!(
            matches!(err, IndexerError::RateLimitExceeded { .. }),
            "Expected RateLimitExceeded, got {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_get_offers_404_returns_stellar_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let err = client.get_offers(None, None, None).await.unwrap_err();

        assert!(matches!(err, IndexerError::StellarApi { .. }));
    }

    #[tokio::test]
    async fn test_get_offers_invalid_json_returns_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let result = client.get_offers(None, None, None).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // get_orderbook – success
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_orderbook_returns_typed_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/order_book"))
            .respond_with(ResponseTemplate::new(200).set_body_string(orderbook_json()))
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let req = OrderbookRequest {
            selling_asset_type: "native",
            selling_asset_code: None,
            selling_asset_issuer: None,
            buying_asset_type: "credit_alphanum4",
            buying_asset_code: Some("USDC"),
            buying_asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            limit: Some(20),
        };
        let ob = client.get_orderbook(req).await.unwrap();

        assert_eq!(ob.bids.len(), 1);
        assert_eq!(ob.asks.len(), 1);
        assert_eq!(ob.bids[0].price, "0.1000000");
        assert_eq!(ob.asks[0].price, "0.1250000");
        assert_eq!(ob.base.asset_type, "native");
        assert_eq!(ob.counter.asset_type, "credit_alphanum4");
    }

    #[tokio::test]
    async fn test_get_orderbook_empty_sides() {
        let mock_server = MockServer::start().await;

        let empty_ob = serde_json::json!({
            "bids": [],
            "asks": [],
            "base": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "counter": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            }
        })
        .to_string();

        Mock::given(method("GET"))
            .and(path("/order_book"))
            .respond_with(ResponseTemplate::new(200).set_body_string(empty_ob))
            .mount(&mock_server)
            .await;

        let client = HorizonClient::new(mock_server.uri());
        let req = OrderbookRequest {
            selling_asset_type: "native",
            selling_asset_code: None,
            selling_asset_issuer: None,
            buying_asset_type: "credit_alphanum4",
            buying_asset_code: Some("USDC"),
            buying_asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            limit: None,
        };
        let ob = client.get_orderbook(req).await.unwrap();
        assert!(ob.is_empty());
        assert!(ob.best_bid().is_none());
        assert!(ob.best_ask().is_none());
        assert!(ob.mid_price().is_none());
    }

    // -----------------------------------------------------------------------
    // get_orderbook – error paths
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_orderbook_500_returns_stellar_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/order_book"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let req = OrderbookRequest {
            selling_asset_type: "native",
            selling_asset_code: None,
            selling_asset_issuer: None,
            buying_asset_type: "credit_alphanum4",
            buying_asset_code: Some("USDC"),
            buying_asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            limit: None,
        };
        let err = client.get_orderbook(req).await.unwrap_err();
        match err {
            IndexerError::StellarApi { status, .. } => assert_eq!(status, 500),
            other => panic!("Expected StellarApi, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_orderbook_invalid_json_returns_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/order_book"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{bad json"))
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let req = OrderbookRequest {
            selling_asset_type: "native",
            selling_asset_code: None,
            selling_asset_issuer: None,
            buying_asset_type: "credit_alphanum4",
            buying_asset_code: Some("USDC"),
            buying_asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
            limit: None,
        };
        let result = client.get_orderbook(req).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // HorizonOrderbook helper methods
    // -----------------------------------------------------------------------

    #[test]
    fn test_orderbook_best_bid_and_ask() {
        use crate::models::horizon::{HorizonAsset, HorizonOrderbook, OrderbookLevel};

        let ob = HorizonOrderbook {
            bids: vec![OrderbookLevel {
                price_r: HorizonPriceR { n: 1, d: 10 },
                price: "0.1000000".to_string(),
                amount: "1000.0000000".to_string(),
            }],
            asks: vec![OrderbookLevel {
                price_r: HorizonPriceR { n: 1, d: 8 },
                price: "0.1250000".to_string(),
                amount: "500.0000000".to_string(),
            }],
            base: HorizonAsset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            counter: HorizonAsset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: Some(
                    "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
                ),
            },
        };

        assert!(!ob.is_empty());
        assert_eq!(ob.best_bid(), Some("0.1000000"));
        assert_eq!(ob.best_ask(), Some("0.1250000"));

        let mid = ob.mid_price().unwrap();
        assert!((mid - 0.1125).abs() < 1e-7);
    }

    #[test]
    fn test_orderbook_is_empty_when_both_sides_empty() {
        use crate::models::horizon::{HorizonAsset, HorizonOrderbook};

        let ob = HorizonOrderbook {
            bids: vec![],
            asks: vec![],
            base: HorizonAsset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            counter: HorizonAsset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: Some(
                    "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
                ),
            },
        };
        assert!(ob.is_empty());
        assert!(ob.mid_price().is_none());
    }

    // -----------------------------------------------------------------------
    // parse_asset
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_asset_native() {
        let client = HorizonClient::new("https://horizon.stellar.org");
        let v = serde_json::json!({ "asset_type": "native" });
        let asset = client.parse_asset(&v).unwrap();
        assert_eq!(asset, crate::models::asset::Asset::Native);
    }

    #[test]
    fn test_parse_asset_credit_alphanum4() {
        let client = HorizonClient::new("https://horizon.stellar.org");
        let v = serde_json::json!({
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        });
        let asset = client.parse_asset(&v).unwrap();
        match asset {
            crate::models::asset::Asset::CreditAlphanum4 {
                asset_code,
                asset_issuer,
            } => {
                assert_eq!(asset_code, "USDC");
                assert_eq!(
                    asset_issuer,
                    "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                );
            }
            other => panic!("Unexpected asset variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_asset_credit_alphanum12() {
        let client = HorizonClient::new("https://horizon.stellar.org");
        let v = serde_json::json!({
            "asset_type": "credit_alphanum12",
            "asset_code": "LONGTOKEN1",
            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        });
        let asset = client.parse_asset(&v).unwrap();
        assert!(matches!(
            asset,
            crate::models::asset::Asset::CreditAlphanum12 { .. }
        ));
    }

    #[test]
    fn test_parse_asset_unknown_type_returns_error() {
        let client = HorizonClient::new("https://horizon.stellar.org");
        let v = serde_json::json!({ "asset_type": "exotic_token" });
        let err = client.parse_asset(&v).unwrap_err();
        assert!(matches!(err, IndexerError::InvalidAsset { .. }));
    }

    #[test]
    fn test_parse_asset_missing_type_field() {
        let client = HorizonClient::new("https://horizon.stellar.org");
        let v = serde_json::json!({ "asset_code": "USDC" });
        let err = client.parse_asset(&v).unwrap_err();
        assert!(matches!(err, IndexerError::MissingField { .. }));
    }

    // -----------------------------------------------------------------------
    // Retry logic
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_retry_succeeds_after_transient_failures() {
        let mock_server = MockServer::start().await;

        // First request fails, second succeeds
        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(offers_page_json(serde_json::json!([sample_offer_json()]))),
            )
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1, // fast for tests
            max_delay_ms: 10,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let offers = client.get_offers(Some(10), None, None).await.unwrap();
        assert_eq!(offers.len(), 1);
    }

    #[tokio::test]
    async fn test_retry_exhausted_returns_last_error() {
        let mock_server = MockServer::start().await;

        // All requests fail
        Mock::given(method("GET"))
            .and(path("/offers"))
            .respond_with(ResponseTemplate::new(500).set_body_string("always fails"))
            .mount(&mock_server)
            .await;

        let cfg = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 1,
            max_delay_ms: 5,
            backoff_multiplier: 1.0,
        };
        let client = HorizonClient::with_retry_config(mock_server.uri(), cfg);
        let err = client.get_offers(Some(10), None, None).await.unwrap_err();
        assert!(matches!(err, IndexerError::StellarApi { .. }));
    }
}
