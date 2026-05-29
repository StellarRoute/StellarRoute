#!/usr/bin/env python3
"""
StellarRoute API Synthetic Monitoring Probes
============================================
Lightweight, zero-dependency Python script to perform synthetic checks against
critical API endpoints: /health, /api/v1/quote, and /api/v1/routes.

Validates:
1. HTTP Response Code (200 OK)
2. Response Latency vs Configured Thresholds
3. JSON Envelope and Payload Schema Shapes & Types

Outputs structured JSON logs and triggers appropriate on-call runbook references on failure.
"""

import argparse
import datetime
import json
import os
import sys
import time
import urllib.request
import urllib.error

# Runbook associations for quick incident response
RUNBOOKS = {
    "health": "https://links.internal/runbooks/m5-health-failure",
    "quote": "https://links.internal/runbooks/m5-quote-failure",
    "routes": "https://links.internal/runbooks/m5-routes-failure",
}


def log_result(endpoint: str, success: bool, latency_ms: float, error_msg: str = None, runbook_url: str = None):
    """Prints a structured JSON log entry for ingestion by log managers (Splunk, Datadog, etc.)."""
    log_entry = {
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
        "endpoint": endpoint,
        "success": success,
        "latency_ms": round(latency_ms, 2) if latency_ms is not None else None,
    }
    if error_msg:
        log_entry["error"] = error_msg
    if runbook_url:
        log_entry["runbook_url"] = runbook_url

    # Always output JSON to stdout so standard log shippers can capture it cleanly
    print(json.dumps(log_entry), flush=True)


def parse_env_defaults():
    """Helper to load environment variables with sane fallback defaults."""
    return {
        "target_url": os.getenv("TARGET_URL", "http://127.0.0.1:3000").rstrip("/"),
        "timeout": float(os.getenv("TIMEOUT_SECONDS", "5.0")),
        "health_threshold": float(os.getenv("HEALTH_THRESHOLD_MS", "200.0")),
        "quote_threshold": float(os.getenv("QUOTE_THRESHOLD_MS", "500.0")),
        "routes_threshold": float(os.getenv("ROUTES_THRESHOLD_MS", "800.0")),
        "base_asset": os.getenv("PROBE_BASE_ASSET", "native"),
        "quote_asset": os.getenv("PROBE_QUOTE_ASSET", "USDC"),
        "amount": os.getenv("PROBE_AMOUNT", "1.0"),
    }


def make_request(url: str, timeout: float, verbose: bool = False) -> tuple[int, bytes, float, str | None]:
    """Helper to execute an HTTP GET request and measure response time precisely."""
    if verbose:
        print(f"Making request to: {url}", file=sys.stderr)
        
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/json",
            "User-Agent": "StellarRoute-SyntheticProbe/1.0",
        }
    )

    start_time = time.perf_counter()
    error_msg = None
    status_code = 0
    response_body = b""

    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            status_code = response.status
            response_body = response.read()
    except urllib.error.HTTPError as e:
        status_code = e.code
        try:
            response_body = e.read()
        except Exception:
            response_body = b""
        error_msg = f"HTTP Error {e.code}: {e.reason}"
    except urllib.error.URLError as e:
        error_msg = f"URL/Connection Error: {e.reason}"
    except Exception as e:
        error_msg = f"Unexpected Error: {str(e)}"

    end_time = time.perf_counter()
    latency_ms = (end_time - start_time) * 1000.0

    return status_code, response_body, latency_ms, error_msg


def validate_envelope(payload: dict) -> str | None:
    """Validates the standard StellarRoute API envelope shape: v, timestamp, request_id, data."""
    if not isinstance(payload, dict):
        return "Root response is not a JSON object"
        
    for field in ["v", "timestamp", "request_id", "data"]:
        if field not in payload:
            return f"Missing envelope field: '{field}'"

    if not isinstance(payload["v"], int):
        return f"Envelope field 'v' must be an integer, got {type(payload['v']).__name__}"
    if not isinstance(payload["timestamp"], int):
        return f"Envelope field 'timestamp' must be an integer, got {type(payload['timestamp']).__name__}"
    if not isinstance(payload["request_id"], str):
        return f"Envelope field 'request_id' must be a string, got {type(payload['request_id']).__name__}"
    
    return None


def check_health(target_url: str, timeout: float, threshold: float, verbose: bool = False) -> bool:
    """Probes /health endpoint, checks latency and system components."""
    url = f"{target_url}/health"
    status_code, body, latency, err = make_request(url, timeout, verbose)

    if err:
        log_result("health", False, latency, err, RUNBOOKS["health"])
        return False

    if status_code != 200:
        log_result("health", False, latency, f"Expected HTTP 200, got {status_code}", RUNBOOKS["health"])
        return False

    # Check performance threshold
    if latency > threshold:
        log_result(
            "health",
            False,
            latency,
            f"Performance degradation: latency {latency:.2f}ms exceeded threshold of {threshold}ms",
            RUNBOOKS["health"]
        )
        return False

    # Check payload structure and contract
    try:
        payload = json.loads(body.decode("utf-8"))
    except Exception as e:
        log_result("health", False, latency, f"Failed to parse JSON body: {e}", RUNBOOKS["health"])
        return False

    envelope_err = validate_envelope(payload)
    if envelope_err:
        log_result("health", False, latency, f"Invalid Envelope: {envelope_err}", RUNBOOKS["health"])
        return False

    data = payload["data"]
    if not isinstance(data, dict):
        log_result("health", False, latency, f"Envelope 'data' must be a JSON object, got {type(data).__name__}", RUNBOOKS["health"])
        return False

    # HealthResponse check
    for field in ["status", "timestamp", "version", "components"]:
        if field not in data:
            log_result("health", False, latency, f"Missing HealthResponse field: '{field}'", RUNBOOKS["health"])
            return False

    if data["status"] != "healthy":
        log_result("health", False, latency, f"System health is degraded: status='{data['status']}'", RUNBOOKS["health"])
        return False

    if not isinstance(data["components"], dict):
        log_result("health", False, latency, f"HealthResponse 'components' must be a JSON object", RUNBOOKS["health"])
        return False

    if verbose:
        print(f"Health check passed successfully. Latency: {latency:.2f}ms", file=sys.stderr)

    log_result("health", True, latency)
    return True


def check_asset_info(asset: dict, asset_name: str) -> str | None:
    """Helper to validate asset information field structure."""
    if not isinstance(asset, dict):
        return f"Asset info '{asset_name}' must be an object"
    if "asset_type" not in asset:
        return f"Asset info '{asset_name}' is missing 'asset_type'"
    if "code" not in asset:
        return f"Asset info '{asset_name}' is missing 'code'"
    return None


def check_quote(
    target_url: str,
    base: str,
    quote: str,
    amount: str,
    timeout: float,
    threshold: float,
    verbose: bool = False
) -> bool:
    """Probes /api/v1/quote/{base}/{quote}?amount={amount} endpoint."""
    url = f"{target_url}/api/v1/quote/{base}/{quote}?amount={amount}"
    status_code, body, latency, err = make_request(url, timeout, verbose)

    if err:
        log_result("quote", False, latency, err, RUNBOOKS["quote"])
        return False

    if status_code != 200:
        log_result("quote", False, latency, f"Expected HTTP 200, got {status_code}", RUNBOOKS["quote"])
        return False

    # Check performance threshold
    if latency > threshold:
        log_result(
            "quote",
            False,
            latency,
            f"Performance degradation: latency {latency:.2f}ms exceeded threshold of {threshold}ms",
            RUNBOOKS["quote"]
        )
        return False

    # Check payload structure and contract
    try:
        payload = json.loads(body.decode("utf-8"))
    except Exception as e:
        log_result("quote", False, latency, f"Failed to parse JSON body: {e}", RUNBOOKS["quote"])
        return False

    envelope_err = validate_envelope(payload)
    if envelope_err:
        log_result("quote", False, latency, f"Invalid Envelope: {envelope_err}", RUNBOOKS["quote"])
        return False

    data = payload["data"]
    if not isinstance(data, dict):
        log_result("quote", False, latency, f"Envelope 'data' must be a JSON object, got {type(data).__name__}", RUNBOOKS["quote"])
        return False

    # QuoteResponse check
    required_fields = ["base_asset", "quote_asset", "amount", "price", "total", "path", "timestamp"]
    for field in required_fields:
        if field not in data:
            log_result("quote", False, latency, f"Missing QuoteResponse field: '{field}'", RUNBOOKS["quote"])
            return False

    # Validate asset info structures
    base_err = check_asset_info(data["base_asset"], "base_asset")
    if base_err:
        log_result("quote", False, latency, f"Invalid base asset info: {base_err}", RUNBOOKS["quote"])
        return False

    quote_err = check_asset_info(data["quote_asset"], "quote_asset")
    if quote_err:
        log_result("quote", False, latency, f"Invalid quote asset info: {quote_err}", RUNBOOKS["quote"])
        return False

    # Validate value fields
    for field in ["amount", "price", "total"]:
        if not isinstance(data[field], str):
            log_result("quote", False, latency, f"QuoteResponse '{field}' must be a string (precise decimal representation)", RUNBOOKS["quote"])
            return False

    # Validate path steps
    if not isinstance(data["path"], list):
        log_result("quote", False, latency, "QuoteResponse 'path' must be a list", RUNBOOKS["quote"])
        return False

    for idx, step in enumerate(data["path"]):
        if not isinstance(step, dict):
            log_result("quote", False, latency, f"QuoteResponse path step [{idx}] is not an object", RUNBOOKS["quote"])
            return False
        for step_field in ["from_asset", "to_asset", "price", "source"]:
            if step_field not in step:
                log_result("quote", False, latency, f"QuoteResponse path step [{idx}] is missing field '{step_field}'", RUNBOOKS["quote"])
                return False
        if not isinstance(step["price"], str):
            log_result("quote", False, latency, f"QuoteResponse path step [{idx}] 'price' must be a string", RUNBOOKS["quote"])
            return False
        if not isinstance(step["source"], str):
            log_result("quote", False, latency, f"QuoteResponse path step [{idx}] 'source' must be a string", RUNBOOKS["quote"])
            return False

    if verbose:
        print(f"Quote check passed successfully. Latency: {latency:.2f}ms", file=sys.stderr)

    log_result("quote", True, latency)
    return True


def check_routes(
    target_url: str,
    base: str,
    quote: str,
    amount: str,
    timeout: float,
    threshold: float,
    verbose: bool = False
) -> bool:
    """Probes /api/v1/routes/{base}/{quote}?amount={amount} endpoint."""
    url = f"{target_url}/api/v1/routes/{base}/{quote}?amount={amount}"
    status_code, body, latency, err = make_request(url, timeout, verbose)

    if err:
        log_result("routes", False, latency, err, RUNBOOKS["routes"])
        return False

    if status_code != 200:
        log_result("routes", False, latency, f"Expected HTTP 200, got {status_code}", RUNBOOKS["routes"])
        return False

    # Check performance threshold
    if latency > threshold:
        log_result(
            "routes",
            False,
            latency,
            f"Performance degradation: latency {latency:.2f}ms exceeded threshold of {threshold}ms",
            RUNBOOKS["routes"]
        )
        return False

    # Check payload structure and contract
    try:
        payload = json.loads(body.decode("utf-8"))
    except Exception as e:
        log_result("routes", False, latency, f"Failed to parse JSON body: {e}", RUNBOOKS["routes"])
        return False

    envelope_err = validate_envelope(payload)
    if envelope_err:
        log_result("routes", False, latency, f"Invalid Envelope: {envelope_err}", RUNBOOKS["routes"])
        return False

    data = payload["data"]
    if not isinstance(data, dict):
        log_result("routes", False, latency, f"Envelope 'data' must be a JSON object, got {type(data).__name__}", RUNBOOKS["routes"])
        return False

    # RoutesResponse check
    required_fields = ["base_asset", "quote_asset", "amount", "routes", "timestamp"]
    for field in required_fields:
        if field not in data:
            log_result("routes", False, latency, f"Missing RoutesResponse field: '{field}'", RUNBOOKS["routes"])
            return False

    # Validate asset info structures
    base_err = check_asset_info(data["base_asset"], "base_asset")
    if base_err:
        log_result("routes", False, latency, f"Invalid base asset info: {base_err}", RUNBOOKS["routes"])
        return False

    quote_err = check_asset_info(data["quote_asset"], "quote_asset")
    if quote_err:
        log_result("routes", False, latency, f"Invalid quote asset info: {quote_err}", RUNBOOKS["routes"])
        return False

    if not isinstance(data["amount"], str):
        log_result("routes", False, latency, "RoutesResponse 'amount' must be a string", RUNBOOKS["routes"])
        return False

    if not isinstance(data["routes"], list):
        log_result("routes", False, latency, "RoutesResponse 'routes' must be a list", RUNBOOKS["routes"])
        return False

    # Validate RouteCandidates
    for idx, candidate in enumerate(data["routes"]):
        if not isinstance(candidate, dict):
            log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] is not an object", RUNBOOKS["routes"])
            return False
            
        candidate_fields = ["estimated_output", "impact_bps", "score", "policy_used", "path"]
        for cand_field in candidate_fields:
            if cand_field not in candidate:
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] is missing field '{cand_field}'", RUNBOOKS["routes"])
                return False
                
        if not isinstance(candidate["estimated_output"], str):
            log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] 'estimated_output' must be a string", RUNBOOKS["routes"])
            return False
        if not isinstance(candidate["impact_bps"], int):
            log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] 'impact_bps' must be an integer", RUNBOOKS["routes"])
            return False
        if not isinstance(candidate["score"], (int, float)):
            log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] 'score' must be a float or integer", RUNBOOKS["routes"])
            return False
            
        # Validate candidate path hops
        if not isinstance(candidate["path"], list):
            log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] 'path' must be a list", RUNBOOKS["routes"])
            return False
            
        for hop_idx, hop in enumerate(candidate["path"]):
            if not isinstance(hop, dict):
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] is not an object", RUNBOOKS["routes"])
                return False
            for hop_field in ["from_asset", "to_asset", "price", "amount_out_of_hop", "fee_bps", "source"]:
                if hop_field not in hop:
                    log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] is missing field '{hop_field}'", RUNBOOKS["routes"])
                    return False
            if not isinstance(hop["price"], str):
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] 'price' must be a string", RUNBOOKS["routes"])
                return False
            if not isinstance(hop["amount_out_of_hop"], str):
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] 'amount_out_of_hop' must be a string", RUNBOOKS["routes"])
                return False
            if not isinstance(hop["fee_bps"], int):
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] 'fee_bps' must be an integer", RUNBOOKS["routes"])
                return False
            if not isinstance(hop["source"], str):
                log_result("routes", False, latency, f"RoutesResponse candidate [{idx}] hop [{hop_idx}] 'source' must be a string", RUNBOOKS["routes"])
                return False

    if verbose:
        print(f"Routes check passed successfully. Latency: {latency:.2f}ms", file=sys.stderr)

    log_result("routes", True, latency)
    return True


def main():
    defaults = parse_env_defaults()

    parser = argparse.ArgumentParser(
        description="StellarRoute API Synthetic Probes Runner",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument(
        "--endpoint",
        choices=["health", "quote", "routes"],
        help="Run a specific probe instead of checking all endpoints."
    )
    parser.add_argument(
        "--target-url",
        default=defaults["target_url"],
        help="API server target base URL (or TARGET_URL env)."
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=defaults["timeout"],
        help="HTTP connection/read timeout in seconds (or TIMEOUT_SECONDS env)."
    )
    parser.add_argument(
        "--health-threshold",
        type=float,
        default=defaults["health_threshold"],
        help="Health check latency threshold in milliseconds (or HEALTH_THRESHOLD_MS env)."
    )
    parser.add_argument(
        "--quote-threshold",
        type=float,
        default=defaults["quote_threshold"],
        help="Quote check latency threshold in milliseconds (or QUOTE_THRESHOLD_MS env)."
    )
    parser.add_argument(
        "--routes-threshold",
        type=float,
        default=defaults["routes_threshold"],
        help="Routes check latency threshold in milliseconds (or ROUTES_THRESHOLD_MS env)."
    )
    parser.add_argument(
        "--base-asset",
        default=defaults["base_asset"],
        help="Base trading asset code or path (or PROBE_BASE_ASSET env)."
    )
    parser.add_argument(
        "--quote-asset",
        default=defaults["quote_asset"],
        help="Quote trading asset code or path (or PROBE_QUOTE_ASSET env)."
    )
    parser.add_argument(
        "--amount",
        default=defaults["amount"],
        help="Trade amount for quote/routes calculation (or PROBE_AMOUNT env)."
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable diagnostic standard error messages."
    )

    args = parser.parse_args()

    # Normalize target URL
    target_url = args.target_url.rstrip("/")

    success = True

    if args.verbose:
        print("Starting synthetic monitoring probes...", file=sys.stderr)
        print(f"Target URL: {target_url}", file=sys.stderr)
        print(f"Timeout: {args.timeout}s", file=sys.stderr)

    if args.endpoint == "health":
        success = check_health(target_url, args.timeout, args.health_threshold, args.verbose)
    elif args.endpoint == "quote":
        success = check_quote(
            target_url, args.base_asset, args.quote_asset, args.amount,
            args.timeout, args.quote_threshold, args.verbose
        )
    elif args.endpoint == "routes":
        success = check_routes(
            target_url, args.base_asset, args.quote_asset, args.amount,
            args.timeout, args.routes_threshold, args.verbose
        )
    else:
        # Run all three in sequence
        health_ok = check_health(target_url, args.timeout, args.health_threshold, args.verbose)
        quote_ok = check_quote(
            target_url, args.base_asset, args.quote_asset, args.amount,
            args.timeout, args.quote_threshold, args.verbose
        )
        routes_ok = check_routes(
            target_url, args.base_asset, args.quote_asset, args.amount,
            args.timeout, args.routes_threshold, args.verbose
        )
        success = health_ok and quote_ok and routes_ok

    if not success:
        if args.verbose:
            print("Synthetic monitoring probes failed.", file=sys.stderr)
        sys.exit(1)

    if args.verbose:
        print("All synthetic monitoring probes passed successfully.", file=sys.stderr)
    sys.exit(0)


if __name__ == "__main__":
    main()
