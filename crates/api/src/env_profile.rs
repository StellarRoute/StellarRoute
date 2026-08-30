//! Shared helpers for detecting the runtime deployment profile.
//!
//! `STELLARROUTE_ENV=production` is the single source of truth for "this is
//! a production deployment" and gates several hardened defaults (CORS,
//! REQUIRE_AUTH, metrics/replay exposure). `REQUIRE_STRICT_CORS=1` lets an
//! operator opt into the production CORS posture outside of a formally
//! "production" environment (e.g. a staging environment that is still
//! internet-reachable).

/// Parse a boolean-ish value the same way across the API (`1`, `true`,
/// `yes`, `on`, case-insensitive).
pub fn parse_bool(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    matches!(v.as_str(), "1" | "true" | "yes" | "on")
}

/// Parse a boolean-ish environment variable. See [`parse_bool`].
pub fn parse_bool_env(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| parse_bool(&value))
        .unwrap_or(false)
}

/// Whether `STELLARROUTE_ENV` is set to `production` (case-insensitive).
pub fn is_production() -> bool {
    std::env::var("STELLARROUTE_ENV")
        .map(|v| v.trim().eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

/// Whether the strict, production-grade CORS policy must be enforced:
/// either we're in production, or an operator explicitly asked for it via
/// `REQUIRE_STRICT_CORS=1`.
pub fn require_strict_cors() -> bool {
    is_production() || parse_bool_env("REQUIRE_STRICT_CORS")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variables are process-global, so serialize tests that touch them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn is_production_true_only_for_production_value() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("STELLARROUTE_ENV");
        assert!(!is_production());

        std::env::set_var("STELLARROUTE_ENV", "production");
        assert!(is_production());

        std::env::set_var("STELLARROUTE_ENV", "PRODUCTION");
        assert!(is_production());

        std::env::set_var("STELLARROUTE_ENV", "staging");
        assert!(!is_production());

        std::env::remove_var("STELLARROUTE_ENV");
    }

    #[test]
    fn require_strict_cors_via_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("STELLARROUTE_ENV");
        std::env::remove_var("REQUIRE_STRICT_CORS");
        assert!(!require_strict_cors());

        std::env::set_var("REQUIRE_STRICT_CORS", "1");
        assert!(require_strict_cors());

        std::env::remove_var("REQUIRE_STRICT_CORS");
    }
}
