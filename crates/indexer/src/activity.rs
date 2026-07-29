//! Contract swap activity ingestion.

use crate::error::{IndexerError, Result};
use crate::soroban::SorobanEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use stellar_xdr::curr::{Limits, ReadXdr, ScVal};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapActivity {
    pub event_id: String,
    pub contract_id: String,
    pub ledger: i64,
    pub ledger_closed_at: Option<DateTime<Utc>>,
    pub paging_token: String,
    pub sender: String,
    pub amount_in: i128,
    pub amount_out: i128,
    pub fee_amount: i128,
    pub route: Value,
    pub source_asset: Option<String>,
    pub destination_asset: Option<String>,
}

pub fn parse_swap_event(event: &SorobanEvent) -> Result<Option<SwapActivity>> {
    if event.event_type != "contract" || !event.in_successful_contract_call {
        return Ok(None);
    }

    let topics: Vec<String> = event
        .topics
        .iter()
        .map(|topic| normalize_topic(topic))
        .collect();

    if !topics.iter().any(|topic| topic == "swap") {
        return Ok(None);
    }

    let sender = topics
        .iter()
        .position(|topic| topic == "swap")
        .and_then(|idx| topics.get(idx + 1))
        .cloned()
        .or_else(|| topics.get(2).cloned())
        .ok_or_else(|| IndexerError::SorobanRpc("swap event missing sender topic".to_string()))?;

    let payload = parse_payload(event)?;
    let ledger_closed_at = DateTime::parse_from_rfc3339(&event.ledger_closed_at)
        .map(|dt| dt.with_timezone(&Utc))
        .ok();

    Ok(Some(SwapActivity {
        event_id: event.id.clone(),
        contract_id: event.contract_id.clone(),
        ledger: event.ledger as i64,
        ledger_closed_at,
        paging_token: event.paging_token.clone(),
        sender,
        amount_in: payload.amount_in,
        amount_out: payload.amount_out,
        fee_amount: payload.fee_amount,
        route: payload.route,
        source_asset: payload.source_asset,
        destination_asset: payload.destination_asset,
    }))
}

pub async fn ingest_swap_events(pool: &PgPool, events: &[SorobanEvent]) -> Result<u64> {
    let mut inserted = 0;

    for event in events {
        let Some(activity) = parse_swap_event(event)? else {
            continue;
        };

        let result = sqlx::query(
            r#"
            INSERT INTO contract_swap_activity (
                event_id,
                contract_id,
                ledger,
                ledger_closed_at,
                paging_token,
                sender,
                amount_in,
                amount_out,
                fee_amount,
                route,
                source_asset,
                destination_asset
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                CAST($7 AS NUMERIC),
                CAST($8 AS NUMERIC),
                CAST($9 AS NUMERIC),
                CAST($10 AS JSONB),
                $11, $12
            )
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(&activity.event_id)
        .bind(&activity.contract_id)
        .bind(activity.ledger)
        .bind(activity.ledger_closed_at)
        .bind(&activity.paging_token)
        .bind(&activity.sender)
        .bind(activity.amount_in.to_string())
        .bind(activity.amount_out.to_string())
        .bind(activity.fee_amount.to_string())
        .bind(activity.route.to_string())
        .bind(&activity.source_asset)
        .bind(&activity.destination_asset)
        .execute(pool)
        .await?;

        inserted += result.rows_affected();
    }

    Ok(inserted)
}

#[derive(Debug)]
struct ParsedPayload {
    amount_in: i128,
    amount_out: i128,
    fee_amount: i128,
    route: Value,
    source_asset: Option<String>,
    destination_asset: Option<String>,
}

fn parse_payload(event: &SorobanEvent) -> Result<ParsedPayload> {
    if let Ok(value) = serde_json::from_str::<Value>(&event.value.xdr) {
        return parse_json_payload(value);
    }

    let scval = ScVal::from_xdr_base64(&event.value.xdr, Limits::none()).map_err(|err| {
        IndexerError::SorobanRpc(format!("failed to decode swap event value XDR: {err}"))
    })?;

    let ScVal::Vec(Some(values)) = scval else {
        return Err(IndexerError::SorobanRpc(
            "swap event value is not a tuple".to_string(),
        ));
    };

    let amount_in = values
        .first()
        .and_then(parse_scval_integer)
        .ok_or_else(|| IndexerError::SorobanRpc("swap event missing amount_in".to_string()))?;
    let amount_out = values
        .get(1)
        .and_then(parse_scval_integer)
        .ok_or_else(|| IndexerError::SorobanRpc("swap event missing amount_out".to_string()))?;
    let fee_amount = values
        .get(2)
        .and_then(parse_scval_integer)
        .ok_or_else(|| IndexerError::SorobanRpc("swap event missing fee_amount".to_string()))?;

    Ok(ParsedPayload {
        amount_in,
        amount_out,
        fee_amount,
        route: json!({ "xdr": event.value.xdr }),
        source_asset: None,
        destination_asset: None,
    })
}

fn parse_json_payload(value: Value) -> Result<ParsedPayload> {
    Ok(ParsedPayload {
        amount_in: json_i128(&value, "amount_in")?,
        amount_out: json_i128(&value, "amount_out")?,
        fee_amount: json_i128_any(&value, &["fee_amount", "fee"])?,
        route: value.get("route").cloned().unwrap_or_else(|| json!({})),
        source_asset: value
            .get("source_asset")
            .and_then(Value::as_str)
            .map(str::to_string),
        destination_asset: value
            .get("destination_asset")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn json_i128(value: &Value, key: &str) -> Result<i128> {
    json_i128_any(value, &[key])
}

fn json_i128_any(value: &Value, keys: &[&str]) -> Result<i128> {
    for key in keys {
        if let Some(raw) = value.get(*key) {
            if let Some(parsed) = raw
                .as_i64()
                .map(i128::from)
                .or_else(|| raw.as_u64().map(|v| v as i128))
                .or_else(|| raw.as_str().and_then(|s| s.parse::<i128>().ok()))
            {
                return Ok(parsed);
            }
        }
    }

    Err(IndexerError::SorobanRpc(format!(
        "swap event missing numeric field {}",
        keys.join("/")
    )))
}

fn normalize_topic(raw: &str) -> String {
    if let Ok(scval) = ScVal::from_xdr_base64(raw, Limits::none()) {
        match scval {
            ScVal::Symbol(symbol) => return symbol.to_string(),
            ScVal::String(value) => return value.to_string(),
            ScVal::Address(address) => return address.to_string(),
            _ => {}
        }
    }

    raw.trim_matches('"').to_string()
}

fn parse_scval_integer(val: &ScVal) -> Option<i128> {
    match val {
        ScVal::I128(parts) => Some(((parts.hi as i128) << 64) | (parts.lo as i128)),
        ScVal::U128(parts) => Some(((parts.hi as i128) << 64) | (parts.lo as i128)),
        ScVal::I64(v) => Some(*v as i128),
        ScVal::U64(v) => Some(*v as i128),
        ScVal::I32(v) => Some(*v as i128),
        ScVal::U32(v) => Some(*v as i128),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soroban::SorobanEventValue;

    fn event(value: &str) -> SorobanEvent {
        SorobanEvent {
            event_type: "contract".to_string(),
            ledger: 42,
            ledger_closed_at: "2026-06-02T10:00:00Z".to_string(),
            contract_id: "CROUTER".to_string(),
            id: "evt-1".to_string(),
            paging_token: "42-1".to_string(),
            topics: vec![
                "StellarRoute".to_string(),
                "swap".to_string(),
                "GTRADER".to_string(),
            ],
            value: SorobanEventValue {
                xdr: value.to_string(),
            },
            in_successful_contract_call: true,
        }
    }

    #[test]
    fn parses_mock_swap_event() {
        let parsed = parse_swap_event(&event(
            r#"{
                "amount_in": "1000",
                "amount_out": 987,
                "fee_amount": 3,
                "route": {"hops": 1},
                "source_asset": "XLM",
                "destination_asset": "USDC"
            }"#,
        ))
        .expect("parse")
        .expect("swap event");

        assert_eq!(parsed.event_id, "evt-1");
        assert_eq!(parsed.sender, "GTRADER");
        assert_eq!(parsed.amount_in, 1000);
        assert_eq!(parsed.amount_out, 987);
        assert_eq!(parsed.fee_amount, 3);
        assert_eq!(parsed.source_asset.as_deref(), Some("XLM"));
        assert_eq!(parsed.destination_asset.as_deref(), Some("USDC"));
    }

    #[test]
    fn ignores_unsuccessful_events() {
        let mut event = event(r#"{"amount_in":1,"amount_out":1,"fee_amount":0}"#);
        event.in_successful_contract_call = false;

        assert!(parse_swap_event(&event).expect("parse").is_none());
    }
}
