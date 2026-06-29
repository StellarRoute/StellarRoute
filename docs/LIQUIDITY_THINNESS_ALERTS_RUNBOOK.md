# Liquidity Thinness Alerts Runbook

## Overview

The **Liquidity Thinness Alerts** system monitors orderbook liquidity and sends webhook notifications when bid/ask depth falls below configured thresholds. Operators use these alerts to detect potential liquidity crises, slippage risks, or venue outages.

### What Gets Monitored

For each configured trading pair, the system evaluates:
1. **Bid depth**: Total base asset available across all bid levels
2. **Ask depth**: Total base asset available across all ask levels

Alerts are triggered per pair when either depth metric drops below its threshold, with configurable cooldowns to prevent alert storms.

---

## Configuration

### Environment Variables

All alert settings are controlled via environment variables with the `LIQUIDITY_THINNESS_ALERT_` prefix:

```bash
# Webhook URL to send alerts (required to enable)
LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL=https://your-webhook-endpoint.example.com/alerts

# Thresholds for trading pairs (JSON string, see example below)
LIQUIDITY_THINNESS_ALERT_THRESHOLDS='{
  "native/USDC": {
    "min_bid_depth": 10000.0,
    "min_ask_depth": 10000.0,
    "cooldown_seconds": 300
  },
  "XLM/USDC": {
    "min_bid_depth": 5000.0,
    "min_ask_depth": 5000.0,
    "cooldown_seconds": 600
  }
}'

# Retry delay for failed webhook calls (default: 2000ms = 2s)
LIQUIDITY_THINNESS_ALERT_RETRY_DELAY_MS=2000
```

### Thresholds JSON Schema

```json
{
  "pair_name": {
    "min_bid_depth": 10000.0,    // Optional: Minimum bid depth (base asset)
    "min_ask_depth": 10000.0,    // Optional: Minimum ask depth (base asset)
    "cooldown_seconds": 300      // Optional: Cooldown after alert (default: 300s)
  }
}
```

### Example Configurations

#### Development (single pair, aggressive)
```bash
LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL=http://localhost:8080/test-webhook
LIQUIDITY_THINNESS_ALERT_THRESHOLDS='{"native/USDC":{"min_bid_depth":100.0,"min_ask_depth":100.0,"cooldown_seconds":60}}'
```

#### Production (multiple pairs, conservative)
```bash
LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL=https://slack-webhook.example.com/T123/B456/abc123
LIQUIDITY_THINNESS_ALERT_THRESHOLDS='{
  "native/USDC": {
    "min_bid_depth": 50000.0,
    "min_ask_depth": 50000.0,
    "cooldown_seconds": 300
  },
  "native/EURC": {
    "min_bid_depth": 30000.0,
    "min_ask_depth": 30000.0,
    "cooldown_seconds": 600
  }
}'
```

---

## Webhook Payload

When an alert is triggered, the system sends a POST request to the configured webhook with the following JSON payload:

```json
{
  "event": "liquidity_thinness",
  "pair": "native/USDC",
  "base_asset": {
    "asset_type": "native"
  },
  "quote_asset": {
    "asset_type": "credit_alphanum4",
    "asset_code": "USDC",
    "asset_issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
  },
  "threshold": {
    "min_bid_depth": 10000.0,
    "min_ask_depth": 10000.0
  },
  "depth_snapshot": {
    "bid_depth": 2500.0,
    "ask_depth": 9500.0,
    "bid_quote_depth": 275.0,
    "ask_quote_depth": 1045.0,
    "bid_levels": 5,
    "ask_levels": 12,
    "best_bid": "0.1100000",
    "best_ask": "0.1150000"
  },
  "timestamp": "2026-06-01T12:00:00Z"
}
```

### Slack Example (Transformation Proxy)

You can use a simple proxy or serverless function to transform the payload for Slack:

```bash
# Example curl to send Slack message (manual test)
curl -X POST "$SLACK_WEBHOOK_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "text": ":warning: *Liquidity Alert* — native/USDC depth dropped below threshold!",
    "blocks": [
      {
        "type": "section",
        "text": {
          "type": "mrkdwn",
          "text": ":warning: *Liquidity Alert* — native/USDC depth dropped below threshold!"
        }
      },
      {
        "type": "divider"
      },
      {
        "type": "section",
        "fields": [
          {"type": "mrkdwn", "text": "*Pair:*\nnative/USDC"},
          {"type": "mrkdwn", "text": "*Timestamp:*\n2026-06-01T12:00:00Z"},
          {"type": "mrkdwn", "text": "*Bid Depth:*\n2500.0 (threshold: 10000.0)"},
          {"type": "mrkdwn", "text": "*Ask Depth:*\n9500.0 (threshold: 10000.0)"}
        ]
      }
    ]
  }'
```

---

## Observability

### Structured Logging

The alerting system emits structured logs at various log levels:

| Level  | Message                          | Context Fields                                  |
|--------|----------------------------------|------------------------------------------------|
| WARN   | Alerting disabled (no webhook)   |                                                 |
| WARN   | Invalid URL, alerts disabled     |                                                 |
| INFO   | Alert dispatched successfully    | pair, bid_depth, ask_depth, threshold_*, status|
| WARN   | Dispatch failed, retrying        | pair, status, error, response_body             |
| ERROR  | Dispatch failed after retry      | pair, status, error, response_body             |

**Query logs with**:
```bash
# All alert-related logs (last 10 minutes)
stern stellarroute-api -s 10m | grep 'liquidity'

# Alerts only
stern stellarroute-api -s 10m | grep 'Alert dispatched'
```

---

## Operational Procedures

### Verify Alerts Are Working

1. Set a low threshold temporarily
2. Trigger an orderbook request for the pair
3. Check logs or webhook receiver

### Temporarily Disable Alerts

To disable alerts without restarting the service:
1. Update the webhook URL to an invalid value (or unset it entirely)
2. Restart the API server

To disable specific pairs:
1. Remove them from the `LIQUIDITY_THINNESS_ALERT_THRESHOLDS` JSON
2. Restart the API server

---

## Incident Response

### Scenario 1: Alerts Not Firing

**Symptoms**: Liquidity drops below threshold but no webhook is received, no logs.

**Diagnosis**:
- Check `LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL` is set and valid
- Check `LIQUIDITY_THINNESS_ALERT_THRESHOLDS` is valid JSON
- Look for WARN logs like "alerts disabled"

**Resolution**:
```bash
# Validate URL
curl -X POST "$LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL" -d '{"test":"hello"}'

# Validate thresholds JSON
echo "$LIQUIDITY_THINNESS_ALERT_THRESHOLDS" | python -m json.tool
```

### Scenario 2: Webhook Returns 4xx Errors

**Symptoms**: ERROR logs with status code 400-499.

**Common causes**:
- Invalid URL or authentication token expired
- Webhook endpoint doesn't accept POST with `application/json`
- Payload schema mismatch

**Resolution**:
1. Verify webhook URL is correct (check for typos, query params)
2. Validate endpoint accepts POST requests with JSON payloads
3. Compare sent payload to expected schema

### Scenario 3: Webhook Returns 5xx Errors

**Symptoms**: WARN/ERROR logs with status code 500-599.

**What happens**:
- System retries once after 2 seconds
- If retry fails, logs ERROR and gives up

**Resolution**:
1. Check webhook receiver logs for errors
2. Verify receiver is up and healthy
3. Consider increasing `LIQUIDITY_THINNESS_ALERT_RETRY_DELAY_MS` if needed

### Scenario 4: Alert Storms (Too Many Alerts)

**Symptoms**: Multiple alerts for same pair in short time.

**Diagnosis**:
- Check cooldown_seconds setting in thresholds
- Look for last_sent_at in logs

**Resolution**:
- Increase cooldown_seconds for affected pairs
```bash
LIQUIDITY_THINNESS_ALERT_THRESHOLDS='{"native/USDC":{"min_bid_depth":10000.0,"cooldown_seconds":1800}}'
```

---

## See Also

- **Implementation**: `crates/api/src/liquidity_alerts.rs`
- **Tests**: `crates/api/tests/liquidity_thinness_alerts_integration.rs`
- **Orderbook Endpoint**: `crates/api/src/routes/orderbook.rs`
