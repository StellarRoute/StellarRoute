#!/usr/bin/env python3
"""
StellarRoute Mock API Server for Synthetic Probe Testing
========================================================
A simple, zero-dependency HTTP server built using Python's standard library.
Used to mock StellarRoute's /health, /api/v1/quote, and /api/v1/routes endpoints
to test the synthetic probe runner.

Supports simulated failure modes via query parameters:
- ?slow=1         -> Introduces latency (e.g. 1.5 seconds)
- ?status=500     -> Returns HTTP 500 status code
- ?corrupt=1      -> Mutates schema shape or types to trigger schema errors
- ?unhealthy=1    -> (For health check) returns status "unhealthy"
"""

import http.server
import json
import socketserver
import time
import urllib.parse

PORT = 8080

# Mock template responses
HEALTH_SUCCESS = {
    "v": 1,
    "timestamp": 1740312000000,
    "request_id": "req_mock_health_01",
    "data": {
        "status": "healthy",
        "timestamp": "2026-05-27T14:10:00Z",
        "version": "0.1.0",
        "components": {
            "database": "healthy",
            "redis": "healthy"
        }
    }
}

QUOTE_SUCCESS = {
    "v": 1,
    "timestamp": 1740312000000,
    "request_id": "req_mock_quote_02",
    "data": {
        "base_asset": {
            "asset_type": "native",
            "code": "XLM",
            "issuer": None
        },
        "quote_asset": {
            "asset_type": "credit_alphanum4",
            "code": "USDC",
            "issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        },
        "amount": "1.0",
        "price": "0.1154200",
        "total": "0.1154200",
        "quote_type": "exact_input",
        "degraded": False,
        "path": [
            {
                "from_asset": {
                    "asset_type": "native",
                    "code": "XLM",
                    "issuer": None
                },
                "to_asset": {
                    "asset_type": "credit_alphanum4",
                    "code": "USDC",
                    "issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
                },
                "price": "0.1154200",
                "source": "sdex"
            }
        ],
        "timestamp": 1740312000000
    }
}

ROUTES_SUCCESS = {
    "v": 1,
    "timestamp": 1740312000000,
    "request_id": "req_mock_routes_03",
    "data": {
        "base_asset": {
            "asset_type": "native",
            "code": "XLM",
            "issuer": None
        },
        "quote_asset": {
            "asset_type": "credit_alphanum4",
            "code": "USDC",
            "issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        },
        "amount": "1.0",
        "routes": [
            {
                "estimated_output": "0.1154200",
                "impact_bps": 0,
                "score": 1.0,
                "policy_used": "production",
                "path": [
                    {
                        "from_asset": {
                            "asset_type": "native",
                            "code": "XLM",
                            "issuer": None
                        },
                        "to_asset": {
                            "asset_type": "credit_alphanum4",
                            "code": "USDC",
                            "issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
                        },
                        "price": "0.1154200",
                        "amount_out_of_hop": "0.1154200",
                        "fee_bps": 30,
                        "source": "sdex"
                    }
                ]
            }
        ],
        "timestamp": 1740312000000
    }
}


class MockAPIRequestHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        # Silence standard HTTP request logging to avoid cluttering test outputs
        pass

    def do_GET(self):
        parsed_url = urllib.parse.urlparse(self.path)
        path = parsed_url.path
        query = urllib.parse.parse_qs(parsed_url.query)

        # 1. Simulate Latency
        if "slow" in query:
            time.sleep(1.2)  # Delay of 1.2 seconds to breach 200/500/800ms thresholds

        # 2. Simulate HTTP Status Failures
        if "status" in query:
            status_code = int(query["status"][0])
            self.send_response(status_code)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": f"Simulated HTTP {status_code}"}).encode("utf-8"))
            return

        # 3. Route Handlers
        if path == "/health":
            self.handle_health(query)
        elif path.startswith("/api/v1/quote/"):
            self.handle_quote(query)
        elif path.startswith("/api/v1/routes/"):
            self.handle_routes(query)
        else:
            self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": "Endpoint not found"}).encode("utf-8"))

    def handle_health(self, query):
        response_data = json.loads(json.dumps(HEALTH_SUCCESS))

        if "unhealthy" in query:
            response_data["data"]["status"] = "unhealthy"
            response_data["data"]["components"]["database"] = "unhealthy"

        if "corrupt" in query:
            # Delete a mandatory field
            del response_data["data"]["status"]

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response_data).encode("utf-8"))

    def handle_quote(self, query):
        response_data = json.loads(json.dumps(QUOTE_SUCCESS))

        if "corrupt" in query:
            # Mutate price type from string to integer
            response_data["data"]["price"] = 123  # Should be string "0.1154200"

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response_data).encode("utf-8"))

    def handle_routes(self, query):
        response_data = json.loads(json.dumps(ROUTES_SUCCESS))

        if "corrupt" in query:
            # Mutate routes key to not be a list
            response_data["data"]["routes"] = "not-a-list"

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response_data).encode("utf-8"))


def main():
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), MockAPIRequestHandler) as httpd:
        print(f"Mock StellarRoute API running on http://127.0.0.1:{PORT}")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
