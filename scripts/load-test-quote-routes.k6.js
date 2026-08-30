// StellarRoute public endpoint load test
// ===========================================================================
// Hits GET /api/v1/quote/{base}/{quote} and GET /api/v1/routes/{base}/{quote}
// under configurable ramp/stage load. Designed to run locally or in CI.
//
// Environment variables:
//   BASE_URL          API base URL (default: http://localhost:3000)
//   PAIRS             Comma-separated pairs, e.g. "XLM/USDC,USDC/XLM" (default below)
//   VUS               Peak virtual users (default: 100)
//   DURATION          Hold duration as k6 duration string (default: 2m)
//   RAMP_DURATION     Ramp-up duration (default: 30s)
//   AMOUNT            Quote amount query param (default: 100.0)
//
// Usage:
//   k6 run scripts/load-test-quote-routes.k6.js
//   k6 run -e BASE_URL=https://api.stellarroute.io -e VUS=250 scripts/load-test-quote-routes.k6.js
//
// Pass/fail criteria are derived from docs/performance_budget.md:
//   - quote p95 latency < 500ms
//   - routes p95 latency < 500ms
//   - overall error rate < 1%
// ===========================================================================

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
const PAIRS = (__ENV.PAIRS || 'XLM/USDC,USDC/XLM,XLM/EURC,EURC/USDC').split(',').map((p) => p.trim());
const VUS = parseInt(__ENV.VUS || '100', 10);
const DURATION = __ENV.DURATION || '2m';
const RAMP_DURATION = __ENV.RAMP_DURATION || '30s';
const AMOUNT = __ENV.AMOUNT || '100.0';

// Custom metrics for the two endpoints (k6 built-in http_req_duration is split by tag).
const quoteLatency = new Trend('quote_req_duration_ms');
const routesLatency = new Trend('routes_req_duration_ms');
const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '10s', target: Math.floor(VUS * 0.1) },   // warm up
    { duration: RAMP_DURATION, target: VUS },                // ramp
    { duration: DURATION, target: VUS },                     // steady state
    { duration: '10s', target: 0 },                          // ramp down
  ],
  thresholds: {
    // Match docs/performance_budget.md for HTTP endpoints.
    quote_req_duration_ms: ['p(95)<500'],
    routes_req_duration_ms: ['p(95)<500'],
    errors: ['rate<0.01'],
    http_req_failed: ['rate<0.01'],
  },
};

function pickPair() {
  return PAIRS[Math.floor(Math.random() * PAIRS.length)];
}

function makeUrl(path) {
  const base = BASE_URL.replace(/\/$/, '');
  return `${base}${path}?amount=${encodeURIComponent(AMOUNT)}`;
}

export default function () {
  const [base, quote] = pickPair().split('/');
  const quotePath = `/api/v1/quote/${encodeURIComponent(base)}/${encodeURIComponent(quote)}`;
  const routesPath = `/api/v1/routes/${encodeURIComponent(base)}/${encodeURIComponent(quote)}`;

  const quoteRes = http.get(makeUrl(quotePath), {
    tags: { endpoint: 'quote' },
    headers: { 'Accept': 'application/json' },
  });
  const quoteOk = check(quoteRes, {
    'quote status is 200': (r) => r.status === 200,
    'quote response time < 500ms': (r) => r.timings.duration < 500,
  });
  quoteLatency.add(quoteRes.timings.duration);
  errorRate.add(!quoteOk);

  const routesRes = http.get(makeUrl(routesPath), {
    tags: { endpoint: 'routes' },
    headers: { 'Accept': 'application/json' },
  });
  const routesOk = check(routesRes, {
    'routes status is 200': (r) => r.status === 200,
    'routes response time < 500ms': (r) => r.timings.duration < 500,
  });
  routesLatency.add(routesRes.timings.duration);
  errorRate.add(!routesOk);

  sleep(Math.random() * 0.5 + 0.1);
}

export function handleSummary(data) {
  const result = {
    metadata: {
      base_url: BASE_URL,
      pairs: PAIRS,
      vus: VUS,
      duration: DURATION,
      ramp_duration: RAMP_DURATION,
      amount: AMOUNT,
      timestamp: new Date().toISOString(),
    },
    thresholds: data.thresholds,
    metrics: {
      http_reqs: data.metrics.http_reqs,
      http_req_duration: data.metrics.http_req_duration,
      quote_req_duration_ms: data.metrics.quote_req_duration_ms,
      routes_req_duration_ms: data.metrics.routes_req_duration_ms,
      errors: data.metrics.errors,
      http_req_failed: data.metrics.http_req_failed,
    },
  };
  return {
    'load-test-results.json': JSON.stringify(result, null, 2),
    stdout: JSON.stringify(result, null, 2),
  };
}
