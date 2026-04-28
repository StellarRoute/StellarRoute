//! Route computation job definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::priority::RequestPriority;

/// Unique stable identifier for a route computation task
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct JobId {
    base_asset: String,
    quote_asset: String,
    amount: String,
    quote_type: String,
}

impl JobId {
    pub fn new(base: &str, quote: &str, amount: &str, quote_type: &str) -> Self {
        Self {
            base_asset: base.to_string(),
            quote_asset: quote.to_string(),
            amount: amount.to_string(),
            quote_type: quote_type.to_string(),
        }
    }

    pub fn as_hash_key(&self) -> String {
        format!(
            "route:{}:{}:{}:{}",
            self.base_asset, self.quote_asset, self.amount, self.quote_type
        )
    }
}

/// Payload for route computation task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteComputationTaskPayload {
    pub base_asset: String,
    pub quote_asset: String,
    pub base_asset_id: Uuid,
    pub quote_asset_id: Uuid,
    pub amount: f64,
    pub slippage_bps: u32,
    pub quote_type: String,
}

/// Route computation job
#[derive(Debug, Clone)]
pub struct RouteComputationJob {
    pub id: JobId,
    pub payload: RouteComputationTaskPayload,
    pub created_at: DateTime<Utc>,
    pub attempt: u32,
    pub max_retries: u32,
    /// Priority band assigned at submission time.
    pub priority: RequestPriority,
    /// WFQ virtual time used by the starvation-prevention scheduler.
    pub virtual_time: i64,
}

impl RouteComputationJob {
    pub fn new(
        base: &str,
        quote: &str,
        payload: RouteComputationTaskPayload,
        max_retries: u32,
    ) -> Self {
        Self {
            id: JobId::new(
                base,
                quote,
                &format!("{:.7}", payload.amount),
                &payload.quote_type,
            ),
            payload,
            created_at: Utc::now(),
            attempt: 0,
            max_retries,
            priority: RequestPriority::Normal,
            virtual_time: 0,
        }
    }

    /// Builder-style setter for priority and virtual time.
    pub fn with_priority(mut self, priority: RequestPriority, virtual_time: i64) -> Self {
        self.priority = priority;
        self.virtual_time = virtual_time;
        self
    }

    pub fn is_exhausted(&self) -> bool {
        self.attempt >= self.max_retries
    }

    pub fn next_attempt(mut self) -> Self {
        self.attempt += 1;
        self
    }
}

/// Result of route computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteComputationResult {
    pub job_id: String,
    pub price: f64,
    pub total: f64,
    pub computed_at: DateTime<Utc>,
    pub ttl_millis: i64,
}
