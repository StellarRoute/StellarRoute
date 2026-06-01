use std::sync::Arc;
use std::time::Duration;

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::{PgPool, Row};
use tracing::{debug, warn};

use crate::models::QuoteExpirationWebhookPayload;

const WEBHOOK_EVENT_NAME: &str = "quote.expired";
const MAX_DELIVERY_RETRIES: usize = 3;
const INITIAL_BACKOFF_MS: u64 = 500;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct QuoteExpirationWebhookService {
    db: PgPool,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookRegistration {
    pub consumer_id: String,
    pub webhook_url: String,
    pub signing_secret: String,
    pub enabled: bool,
}

impl QuoteExpirationWebhookService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            client: reqwest::Client::new(),
        }
    }

    async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS consumer_quote_expiration_webhooks (
                consumer_id TEXT PRIMARY KEY,
                webhook_url TEXT NOT NULL,
                signing_secret TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn upsert_registration(
        &self,
        consumer_id: &str,
        webhook_url: &str,
        signing_secret: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        self.ensure_schema().await?;

        sqlx::query(
            r#"
            INSERT INTO consumer_quote_expiration_webhooks (
                consumer_id,
                webhook_url,
                signing_secret,
                enabled,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (consumer_id)
            DO UPDATE SET
                webhook_url = EXCLUDED.webhook_url,
                signing_secret = EXCLUDED.signing_secret,
                enabled = EXCLUDED.enabled,
                updated_at = NOW()
            "#,
        )
        .bind(consumer_id)
        .bind(webhook_url)
        .bind(signing_secret)
        .bind(enabled)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_registration(
        &self,
        consumer_id: &str,
    ) -> Result<Option<WebhookRegistration>, sqlx::Error> {
        self.ensure_schema().await?;

        let row = sqlx::query(
            r#"
            SELECT consumer_id, webhook_url, signing_secret, enabled
            FROM consumer_quote_expiration_webhooks
            WHERE consumer_id = $1
            "#,
        )
        .bind(consumer_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| WebhookRegistration {
            consumer_id: r.get("consumer_id"),
            webhook_url: r.get("webhook_url"),
            signing_secret: r.get("signing_secret"),
            enabled: r.get("enabled"),
        }))
    }

    async fn list_enabled_registrations(&self) -> Result<Vec<WebhookRegistration>, sqlx::Error> {
        self.ensure_schema().await?;

        let rows = sqlx::query(
            r#"
            SELECT consumer_id, webhook_url, signing_secret, enabled
            FROM consumer_quote_expiration_webhooks
            WHERE enabled = TRUE
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| WebhookRegistration {
                consumer_id: r.get("consumer_id"),
                webhook_url: r.get("webhook_url"),
                signing_secret: r.get("signing_secret"),
                enabled: r.get("enabled"),
            })
            .collect())
    }

    fn sign_payload(&self, signing_secret: &str, payload_json: &str) -> Option<String> {
        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).ok()?;
        mac.update(payload_json.as_bytes());
        let sig = mac.finalize().into_bytes();
        Some(hex::encode(sig))
    }

    async fn deliver_with_retry(
        &self,
        registration: &WebhookRegistration,
        payload: &QuoteExpirationWebhookPayload,
    ) {
        let payload_json = match serde_json::to_string(payload) {
            Ok(json) => json,
            Err(e) => {
                warn!(error = %e, "Failed to serialize webhook payload");
                return;
            }
        };

        let signature = match self.sign_payload(&registration.signing_secret, &payload_json) {
            Some(sig) => sig,
            None => {
                warn!(
                    consumer_id = %registration.consumer_id,
                    "Failed to sign webhook payload"
                );
                return;
            }
        };

        for attempt in 0..=MAX_DELIVERY_RETRIES {
            let response = self
                .client
                .post(&registration.webhook_url)
                .header("content-type", "application/json")
                .header("x-stellarroute-event", WEBHOOK_EVENT_NAME)
                .header("x-stellarroute-consumer", &registration.consumer_id)
                .header("x-stellarroute-signature", format!("sha256={signature}"))
                .body(payload_json.clone())
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    debug!(
                        consumer_id = %registration.consumer_id,
                        status = %resp.status(),
                        "Webhook delivered"
                    );
                    return;
                }
                Ok(resp) => {
                    warn!(
                        consumer_id = %registration.consumer_id,
                        status = %resp.status(),
                        attempt,
                        "Webhook delivery failed"
                    );
                }
                Err(e) => {
                    warn!(
                        consumer_id = %registration.consumer_id,
                        error = %e,
                        attempt,
                        "Webhook delivery request error"
                    );
                }
            }

            if attempt < MAX_DELIVERY_RETRIES {
                let backoff_ms = INITIAL_BACKOFF_MS.saturating_mul(1u64 << attempt);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    pub async fn dispatch_to_consumer(
        &self,
        consumer_id: &str,
        payload: QuoteExpirationWebhookPayload,
    ) {
        match self.get_registration(consumer_id).await {
            Ok(Some(registration)) if registration.enabled => {
                self.deliver_with_retry(&registration, &payload).await;
            }
            Ok(Some(_)) => {
                debug!(consumer_id, "Webhook disabled; skipping dispatch");
            }
            Ok(None) => {
                debug!(consumer_id, "No webhook registration for consumer");
            }
            Err(e) => {
                warn!(consumer_id, error = %e, "Failed to load webhook registration");
            }
        }
    }

    pub async fn dispatch_to_all(&self, payload: QuoteExpirationWebhookPayload) {
        match self.list_enabled_registrations().await {
            Ok(registrations) => {
                for registration in registrations {
                    let consumer_payload = QuoteExpirationWebhookPayload {
                        consumer_id: registration.consumer_id.clone(),
                        ..payload.clone()
                    };
                    self.deliver_with_retry(&registration, &consumer_payload)
                        .await;
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to list enabled webhook registrations");
            }
        }
    }

    pub fn spawn_delayed_dispatch_for_consumer(
        self: Arc<Self>,
        consumer_id: String,
        payload: QuoteExpirationWebhookPayload,
        delay: Duration,
    ) {
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            self.dispatch_to_consumer(&consumer_id, payload).await;
        });
    }

    pub fn spawn_dispatch_to_all(self: Arc<Self>, payload: QuoteExpirationWebhookPayload) {
        tokio::spawn(async move {
            self.dispatch_to_all(payload).await;
        });
    }
}
