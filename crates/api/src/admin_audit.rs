use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Simple structured admin audit entry used for admin operations (cache flush,
/// kill-switch changes, etc.). Written as JSON to stdout by default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuditEntry {
    pub event: String,
    pub request_id: String,
    pub actor: String,
    pub resource: String,
    pub outcome: String,
    pub timestamp: DateTime<Utc>,
}

/// Build an `AdminAuditEntry` with redacted actor information.
pub fn build_admin_audit_entry(
    event: impl Into<String>,
    request_id: impl Into<String>,
    headers: &HeaderMap,
    resource: impl Into<String>,
    outcome: impl Into<String>,
) -> AdminAuditEntry {
    let actor_raw = extract_actor(headers);
    let actor = redact_actor(&actor_raw);

    AdminAuditEntry {
        event: event.into(),
        request_id: request_id.into(),
        actor,
        resource: resource.into(),
        outcome: outcome.into(),
        timestamp: Utc::now(),
    }
}

/// Emit the audit entry as JSON to stdout when `ADMIN_AUDIT_ENABLED` is not set
/// to "false"/"0". Returns the serialized JSON string on success.
pub fn emit_admin_audit(entry: &AdminAuditEntry) -> Option<String> {
    let enabled = std::env::var("ADMIN_AUDIT_ENABLED")
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true);

    let json = match serde_json::to_string(entry) {
        Ok(j) => j,
        Err(_) => return None,
    };

    if enabled {
        println!("{}", json);
    }

    Some(json)
}

/// Redact actor secrets. Expected input formats:
/// - "apikey:actual-secret" => "apikey:[REDACTED]"
/// - "token:the-token" => "token:[REDACTED]"
/// - "ip:1.2.3.4" => unchanged
fn redact_actor(actor: &str) -> String {
    if actor.starts_with("apikey:") {
        "apikey:[REDACTED]".to_string()
    } else if actor.starts_with("token:") {
        "token:[REDACTED]".to_string()
    } else {
        actor.to_string()
    }
}

/// Extract an actor string from common auth headers, falling back to IP.
/// Mirrors the logic in `middleware::rate_limit::extract_identity` but returns
/// a simple string suitable for audit logging.
fn extract_actor(headers: &HeaderMap) -> String {
    // X-API-Key
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return format!("apikey:{}", key);
    }

    // Authorization: Bearer token
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(tok) = auth.strip_prefix("Bearer ") {
            return format!("token:{}", tok);
        }
    }

    // Fallback to IP from forwarding headers
    // X-Forwarded-For
    if let Some(fwd) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = fwd.split(',').next() {
            return format!("ip:{}", first.trim());
        }
    }

    if let Some(real) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return format!("ip:{}", real.trim());
    }

    // Unknown — return loopback
    "ip:127.0.0.1".to_string()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn redact_apikey_and_token() {
        let mut hm = HeaderMap::new();
        hm.insert("x-api-key", "secret-key-123".parse().unwrap());
        let entry = build_admin_audit_entry("cache.flush", "req-1", &hm, "pairs", "success");
        assert_eq!(entry.actor, "apikey:[REDACTED]");

        let mut hm2 = HeaderMap::new();
        hm2.insert("authorization", "Bearer mytoken".parse().unwrap());
        let entry2 = build_admin_audit_entry("ks.update", "req-2", &hm2, "kill_switch", "denied");
        assert_eq!(entry2.actor, "token:[REDACTED]");
    }

    #[test]
    fn json_emitted_when_enabled() {
        std::env::remove_var("ADMIN_AUDIT_ENABLED");
        let hm = HeaderMap::new();
        let entry = build_admin_audit_entry("cache.flush", "req-3", &hm, "pairs", "success");
        let json = emit_admin_audit(&entry).expect("json");
        assert!(json.contains("\"event\":\"cache.flush\""));
        assert!(json.contains("\"request_id\":\"req-3\""));
    }

    #[test]
    fn actor_falls_back_to_ip() {
        let mut hm = HeaderMap::new();
        hm.insert("x-forwarded-for", "10.0.0.1, 1.2.3.4".parse().unwrap());
        let entry = build_admin_audit_entry("cache.flush", "req-4", &hm, "all", "success");
        assert!(entry.actor.starts_with("ip:"));
    }
}
