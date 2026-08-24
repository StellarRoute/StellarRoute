#!/usr/bin/env python3
"""
StellarRoute Canary Live Quote Comparison
=========================================
Zero-dependency Python script that continuously cross-checks StellarRoute
quote prices against Stellar Horizon's SDEX order-book API.

For each monitored pair:
  1. Fetches GET /api/v1/quote/{base}/{quote}?amount={amount} from StellarRoute.
  2. Fetches GET {horizon}/order_book?... to obtain the Horizon best-ask price.
  3. Computes Divergence_BPS = abs(sr_price - ref_price) / ref_price * 10_000.
  4. Logs a structured JSON result to stdout.
  5. POSTs the result to POST /api/v1/system/canary/live-compare (fire-and-forget).
  6. Tracks consecutive divergence failures; exits 1 on Sustained_Divergence.

See docs/routing_canary.md — "Live Quote Comparison Job" for full context.
"""

import argparse
import datetime
import json
import math
import os
import sys
import time
import urllib.error
import urllib.request

# Runbook URL injected into alert log entries.
RUNBOOK_URL = "https://links.internal/runbooks/canary-divergence"


# ---------------------------------------------------------------------------
# Configuration helpers
# ---------------------------------------------------------------------------

def _env(name: str, default: str) -> str:
    return os.environ.get(name, default)


def _env_bool(name: str, default: bool) -> bool:
    val = os.environ.get(name, "").strip().lower()
    if not val:
        return default
    return val in ("1", "true", "yes", "on")


def parse_config(argv=None):
    """Parse configuration from env vars + CLI args (CLI takes precedence)."""
    defaults = {
        "sr_base_url": _env("CANARY_SR_BASE_URL", "http://localhost:3000"),
        "horizon_base_url": _env("CANARY_HORIZON_BASE_URL", "https://horizon.stellar.org"),
        "base_asset": _env("CANARY_BASE_ASSET", "native"),
        "quote_asset": _env(
            "CANARY_QUOTE_ASSET",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        ),
        "amount": _env("CANARY_AMOUNT", "1000.0"),
        "timeout": float(_env("CANARY_TIMEOUT", "10.0")),
        "divergence_threshold": int(_env("CANARY_DIVERGENCE_THRESHOLD_BPS", "50")),
        "failure_threshold": int(_env("CANARY_FAILURE_THRESHOLD", "3")),
        "admin_token": _env("CANARY_ADMIN_TOKEN", ""),
        "count_errors_as_failures": _env_bool("CANARY_COUNT_ERRORS_AS_FAILURES", False),
    }

    parser = argparse.ArgumentParser(
        description="StellarRoute Canary Live Quote Comparison",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--sr-base-url", default=defaults["sr_base_url"],
                        help="StellarRoute API base URL (env: CANARY_SR_BASE_URL)")
    parser.add_argument("--horizon-base-url", default=defaults["horizon_base_url"],
                        help="Horizon API base URL (env: CANARY_HORIZON_BASE_URL)")
    parser.add_argument("--base-asset", default=defaults["base_asset"],
                        help="Selling asset: 'native' or 'CODE:ISSUER' (env: CANARY_BASE_ASSET)")
    parser.add_argument("--quote-asset", default=defaults["quote_asset"],
                        help="Buying asset: 'CODE:ISSUER' (env: CANARY_QUOTE_ASSET)")
    parser.add_argument("--amount", default=defaults["amount"],
                        help="Trade size for StellarRoute quote (env: CANARY_AMOUNT)")
    parser.add_argument("--timeout", type=float, default=defaults["timeout"],
                        help="HTTP timeout in seconds (env: CANARY_TIMEOUT)")
    parser.add_argument("--divergence-threshold", type=int,
                        default=defaults["divergence_threshold"],
                        help="BPS above which a run is 'diverged' (env: CANARY_DIVERGENCE_THRESHOLD_BPS)")
    parser.add_argument("--failure-threshold", type=int,
                        default=defaults["failure_threshold"],
                        help="Consecutive failures before exit 1 (env: CANARY_FAILURE_THRESHOLD)")
    parser.add_argument("--admin-token", default=defaults["admin_token"],
                        help="Bearer token for POST /api/v1/system/canary/live-compare (env: CANARY_ADMIN_TOKEN)")
    parser.add_argument("--count-errors-as-failures", action="store_true",
                        default=defaults["count_errors_as_failures"],
                        help="Count HTTP errors toward consecutive failure threshold (env: CANARY_COUNT_ERRORS_AS_FAILURES)")
    parser.add_argument("--verbose", action="store_true",
                        help="Emit diagnostic messages to stderr")

    return parser.parse_args(argv)


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _now_iso() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")


def _get(url: str, timeout: float, headers: dict = None):
    """Execute an HTTP GET. Returns (status_code, body_bytes, error_str)."""
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/json",
            "User-Agent": "StellarRoute-CanaryCompare/1.0",
            **(headers or {}),
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, resp.read(), None
    except urllib.error.HTTPError as e:
        body = b""
        try:
            body = e.read()
        except Exception:
            pass
        return e.code, body, f"HTTP {e.code}: {e.reason}"
    except Exception as e:
        return 0, b"", str(e)


def _post_json(url: str, payload: dict, token: str, timeout: float):
    """POST JSON payload. Returns (status_code, error_str)."""
    data = json.dumps(payload).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "User-Agent": "StellarRoute-CanaryCompare/1.0",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, data=data, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, None
    except urllib.error.HTTPError as e:
        return e.code, f"HTTP {e.code}: {e.reason}"
    except Exception as e:
        return 0, str(e)


# ---------------------------------------------------------------------------
# Price fetching
# ---------------------------------------------------------------------------

def fetch_sr_price(base_url: str, base_asset: str, quote_asset: str,
                   amount: str, timeout: float, verbose: bool):
    """
    Fetch a quote from StellarRoute.
    Returns (price_str, error_str). Exactly one of them will be None.
    """
    url = f"{base_url.rstrip('/')}/api/v1/quote/{base_asset}/{quote_asset}?amount={amount}"
    if verbose:
        print(f"[SR] GET {url}", file=sys.stderr)

    status, body, err = _get(url, timeout)
    if err:
        return None, f"StellarRoute unreachable: {err}"
    if status != 200:
        return None, f"StellarRoute returned HTTP {status} for {url}"

    try:
        payload = json.loads(body.decode("utf-8"))
        price = payload["data"]["price"]
        if not isinstance(price, str) or not price:
            return None, "StellarRoute response missing data.price string"
        return price, None
    except Exception as e:
        return None, f"StellarRoute JSON parse error: {e}"


def _build_horizon_orderbook_url(horizon_base: str, base_asset: str, quote_asset: str) -> str:
    """
    Construct the Horizon order_book URL.

    Currently supports:
      - base_asset = "native"  +  quote_asset = "CODE:ISSUER"

    For other combinations the function returns None (unsupported).
    """
    base = base_asset.strip()
    quote = quote_asset.strip()

    if base.lower() != "native":
        return None  # non-native base not supported yet

    if ":" not in quote:
        return None  # native/native not a valid DEX pair

    parts = quote.split(":", 1)
    code, issuer = parts[0], parts[1]
    asset_type = "credit_alphanum4" if len(code) <= 4 else "credit_alphanum12"

    root = horizon_base.rstrip("/")
    return (
        f"{root}/order_book"
        f"?selling_asset_type=native"
        f"&buying_asset_type={asset_type}"
        f"&buying_asset_code={code}"
        f"&buying_asset_issuer={issuer}"
        f"&limit=5"
    )


def fetch_horizon_price(horizon_base: str, base_asset: str, quote_asset: str,
                        timeout: float, verbose: bool):
    """
    Fetch the Horizon best-ask price for the pair.
    Returns (price_str, error_str). Exactly one of them will be None.
    """
    url = _build_horizon_orderbook_url(horizon_base, base_asset, quote_asset)
    if url is None:
        return None, f"Unsupported pair for Horizon lookup: {base_asset}/{quote_asset}"

    if verbose:
        print(f"[Horizon] GET {url}", file=sys.stderr)

    status, body, err = _get(url, timeout)
    if err:
        return None, f"Horizon unreachable: {err}"
    if status != 200:
        return None, f"Horizon returned HTTP {status}"

    try:
        data = json.loads(body.decode("utf-8"))
        asks = data.get("asks", [])
        if not asks:
            return None, "Horizon order book has no asks (empty market)"
        price = asks[0].get("price", "")
        if not price:
            return None, "Horizon order book ask entry missing 'price' field"
        return str(price), None
    except Exception as e:
        return None, f"Horizon JSON parse error: {e}"


# ---------------------------------------------------------------------------
# Comparison logic
# ---------------------------------------------------------------------------

def compute_divergence_bps(sr_price: str, ref_price: str) -> float:
    """
    Compute abs(sr - ref) / ref * 10_000, rounded to 2 decimal places.
    Returns 0.0 if ref_price is zero.
    """
    sr = float(sr_price)
    ref = float(ref_price)
    if ref == 0.0:
        return 0.0
    return round(abs(sr - ref) / ref * 10_000, 2)


# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

def emit_log(pair: str, sr_price, ref_price, divergence_bps, outcome: str,
             consecutive_failures: int, extra: dict = None):
    """Emit one structured JSON log line to stdout."""
    entry = {
        "timestamp": _now_iso(),
        "pair": pair,
        "stellarroute_price": sr_price or "",
        "reference_price": ref_price or "",
        "divergence_bps": divergence_bps if divergence_bps is not None else None,
        "outcome": outcome,
        "consecutive_failures": consecutive_failures,
    }
    if extra:
        entry.update(extra)
    print(json.dumps(entry), flush=True)


def emit_alert_log(pair: str, consecutive_failures: int, threshold: int):
    """Emit the alert JSON log entry (with alert=true)."""
    entry = {
        "timestamp": _now_iso(),
        "alert": True,
        "pair": pair,
        "message": (
            f"Sustained divergence: {consecutive_failures} consecutive failures "
            f"exceeded threshold of {threshold}"
        ),
        "runbook_url": RUNBOOK_URL,
    }
    print(json.dumps(entry), flush=True)


# ---------------------------------------------------------------------------
# API ingest (fire-and-forget)
# ---------------------------------------------------------------------------

def post_result_to_api(base_url: str, token: str, pair: str,
                       sr_price: str, ref_price: str,
                       divergence_bps: float, outcome: str,
                       timestamp: str, timeout: float, verbose: bool):
    """POST a Live_Compare_Result to POST /api/v1/system/canary/live-compare."""
    if not token:
        if verbose:
            print("[API] No admin token configured — skipping ingest POST", file=sys.stderr)
        return

    url = f"{base_url.rstrip('/')}/api/v1/system/canary/live-compare"
    payload = {
        "pair": pair,
        "stellarroute_price": sr_price or "",
        "reference_price": ref_price or "",
        "divergence_bps": divergence_bps if divergence_bps is not None else 0.0,
        "outcome": outcome,
        "timestamp": timestamp,
    }

    if verbose:
        print(f"[API] POST {url}", file=sys.stderr)

    status, err = _post_json(url, payload, token, timeout)
    if err or status not in (200, 201):
        # Fire-and-forget: log warning but do not change exit code.
        warning = {
            "timestamp": _now_iso(),
            "level": "warning",
            "message": f"Failed to POST result to API: {err or f'HTTP {status}'}",
            "url": url,
        }
        print(json.dumps(warning), flush=True)
    elif verbose:
        print(f"[API] Ingest accepted (HTTP {status})", file=sys.stderr)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    cfg = parse_config()

    if cfg.verbose:
        print("StellarRoute Canary Live Quote Comparison starting", file=sys.stderr)
        print(f"  SR base:    {cfg.sr_base_url}", file=sys.stderr)
        print(f"  Horizon:    {cfg.horizon_base_url}", file=sys.stderr)
        print(f"  Pair:       {cfg.base_asset}/{cfg.quote_asset}", file=sys.stderr)
        print(f"  Amount:     {cfg.amount}", file=sys.stderr)
        print(f"  Threshold:  {cfg.divergence_threshold} bps", file=sys.stderr)
        print(f"  Failures:   {cfg.failure_threshold} consecutive", file=sys.stderr)

    pair = f"{cfg.base_asset}/{cfg.quote_asset}"
    consecutive_failures = 0
    timestamp = _now_iso()

    # Fetch StellarRoute price
    sr_price, sr_err = fetch_sr_price(
        cfg.sr_base_url, cfg.base_asset, cfg.quote_asset,
        cfg.amount, cfg.timeout, cfg.verbose,
    )

    # Fetch Horizon reference price
    ref_price, ref_err = fetch_horizon_price(
        cfg.horizon_base_url, cfg.base_asset, cfg.quote_asset,
        cfg.timeout, cfg.verbose,
    )

    # Determine outcome
    if sr_err or ref_err:
        outcome = "error"
        divergence_bps = None
        error_detail = sr_err or ref_err
        emit_log(pair, sr_price, ref_price, divergence_bps, outcome,
                 consecutive_failures, {"error": error_detail})
        if cfg.count_errors_as_failures:
            consecutive_failures += 1
        # else: errors don't count toward consecutive failure threshold
    else:
        divergence_bps = compute_divergence_bps(sr_price, ref_price)
        if divergence_bps > cfg.divergence_threshold:
            outcome = "diverged"
            consecutive_failures += 1
        else:
            outcome = "ok"
            consecutive_failures = 0
        emit_log(pair, sr_price, ref_price, divergence_bps, outcome, consecutive_failures)

    # Push result to API (fire-and-forget)
    post_result_to_api(
        cfg.sr_base_url, cfg.admin_token, pair,
        sr_price, ref_price, divergence_bps, outcome,
        timestamp, cfg.timeout, cfg.verbose,
    )

    # Check for sustained divergence
    if consecutive_failures >= cfg.failure_threshold:
        emit_alert_log(pair, consecutive_failures, cfg.failure_threshold)
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()
