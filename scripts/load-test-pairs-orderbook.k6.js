// StellarRoute pairs + orderbook load test
// ===========================================================================
// Hits GET /api/v1/pairs and GET /api/v1/orderbook/{base}/{quote}
// under configurable ramp/stage load. Designed to run locally or in CI.
//
// Environment variables:
//   BASE_URL          API base URL (default: http://localhost:3000)
//   ORDERBOOK_PAIRS   Comma-separated pairs, e.g. "native/USDC,USDC/XLM" (default below)
//   VUS               Peak virtual users (default: 100)
//   DURATION          Hold duration as k6 duration string (default: 2m)
//   RAMP_DURATION     Ramp-up duration (default: 30s)
//
// Usage:
//   k6 run scripts/load-test-pairs-orderbook.k6.js
//   k6 run -e BASE_URL=https://api.stellarroute.io -e VUS=250 scripts/load-test-pairs-orderbook.k6.js
//
// Pass/fail criteria are derived from docs/performance_budget.md:
//   - pairs p95 latency < 300ms
//   - orderbook p95 latency < 500ms
//   - overall error rate < 1%
// ===========================================================================

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
const ORDERBOOK_PAIRS = (__ENV.ORDERBOOK_PAIRS || 'native/USDC,USDC/XLM,XLM/USDC,EURC/USDC').split(',').map((p) => p.trim());
const VUS = parseInt(__ENV.VUS || '100', 10);
const DURATION = __ENV.DURATION || '2m';
const RAMP_DURATION = __ENV.RAMP_DURATION || '30s';

// Custom metrics for the two endpoints
const pairsLatency = new Trend('pairs_req_duration_ms');
const orderbookLatency = new Trend('orderbook_req_duration_ms');
const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '10s', target: Math.floor(VUS * 0.1) },   // warm up
    { duration: RAMP_DURATION, target: VUS },                // ramp
    { duration: DURATION, target: VUS },                     // steady state
    { duration: '10s', target: 0 },                          // ramp down
  ],
  thresholds: {
    // Match docs/performance_budget.md for HTTP endpoints
    pairs_req_duration_ms: ['p(95)<300'],
    orderbook_req_duration_ms: ['p(95)<500'],
    errors: ['rate<0.01'],
    http_req_failed: ['rate<0.01'],
  },
};

function pickOrderbookPair() {
  return ORDERBOOK_PAIRS[Math.floor(Math.random() * ORDERBOOK_PAIRS.length)];
}

function makeUrl(path) {
  const base = BASE_URL.replace(/\/$/, '');
  return base + path;
}

export default function () {
  // 1. Pairs endpoint - single call per VU iteration
  const pairsPath = '/api/v1/pairs';
  const pairsUrl = makeUrl(pairsPath);
  const pairsRes = http.get(pairsUrl, {
    tags: { endpoint: 'pairs' },
    headers: { 'Accept': 'application/json' },
  });
  const pairsOk = check(pairsRes, {
    'pairs status is 200': (r) => r.status === 200,
    'pairs response time < 300ms': (r) => r.timings.duration < 300,
    'pairs has valid JSON': (r) => {
      try { JSON.parse(r.body); return true; } catch { return false; }
    },
  });
  pairsLatency.add(pairsRes.timings.duration);
  errorRate.add(!pairsOk);

  // 2. Orderbook endpoint - one random pair per VU iteration
  const [base, quote] = pickOrderbookPair().split('/');
  const orderbookPath = '/api/v1/orderbook/' + encodeURIComponent(base) + '/' + encodeURIComponent(quote);
  const orderbookUrl = makeUrl(orderbookPath);
  const orderbookRes = http.get(orderbookUrl, {
    tags: { endpoint: 'orderbook' },
    headers: { 'Accept': 'application/json' },
  });
  const orderbookOk = check(orderbookRes, {
    'orderbook status is 200': (r) => r.status === 200,
    'orderbook response time < 500ms': (r) => r.timings.duration < 500,
    'orderbook has bids and asks': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body.bids) && Array.isArray(body.asks);
      } catch { return false; }
    },
  });
  orderbookLatency.add(orderbookRes.timings.duration);
  errorRate.add(!orderbookOk);

  sleep(Math.random() * 0.5 + 0.1);
}

export function handleSummary(data) {
  const result = {
    metadata: {
      base_url: BASE_URL,
      orderbook_pairs: ORDERBOOK_PAIRS,
      vus: VUS,
      duration: DURATION,
      ramp_duration: RAMP_DURATION,
      timestamp: new Date().toISOString(),
    },
    thresholds: data.thresholds,
    metrics: {
      http_reqs: data.metrics.http_reqs,
      http_req_duration: data.metrics.http_req_duration,
      pairs_req_duration_ms: data.metrics.pairs_req_duration_ms,
      orderbook_req_duration_ms: data.metrics.orderbook_req_duration_ms,
      errors: data.metrics.errors,
      http_req_failed: data.metrics.http_req_failed,
    },
  };
  return {
    'load-test-pairs-orderbook-results.json': JSON.stringify(result, null, 2),
    stdout: JSON.stringify(result, null, 2),
  };
}