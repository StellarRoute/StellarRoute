#!/usr/bin/env node
/**
 * Live swap API smoke — dry-run testnet quote + prepare only.
 *
 * Proves the classic one-hop prepare contract without secrets:
 *   1. quote  — GET /api/v1/quote/:base/:quote
 *   2. prepare — POST /api/v1/swap/prepare
 *
 * Asserts:
 *   - network/passphrase/URL are testnet (rejects public/mainnet)
 *   - prepare returns execution_mode: classic_path_payment
 *   - xdr_envelope is non-empty and is not a transitional placeholder
 *
 * Full Freighter submit/confirm is manual (see checklist). This script does
 * **not** accept STELLAR_SECRET_KEY or perform submit.
 *
 * Usage:
 *   STELLARROUTE_API_URL=http://localhost:8080 \
 *   STELLARROUTE_SENDER=G... \
 *   node scripts/live-swap-api-smoke.mjs
 */

import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const API_URL = (process.env.STELLARROUTE_API_URL ?? 'http://localhost:8080').replace(
  /\/$/,
  '',
);
const BASE = process.env.STELLARROUTE_SMOKE_BASE ?? 'native';
const QUOTE = process.env.STELLARROUTE_SMOKE_QUOTE ?? 'USDC';
const AMOUNT = process.env.STELLARROUTE_SMOKE_AMOUNT ?? '1';
const SENDER =
  process.env.STELLARROUTE_SENDER ??
  'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
const NETWORK = (process.env.STELLAR_NETWORK ?? 'testnet').toLowerCase();
const PASSPHRASE =
  process.env.STELLAR_NETWORK_PASSPHRASE ?? 'Test SDF Network ; September 2015';
const HORIZON_URL = (
  process.env.STELLAR_HORIZON_URL ?? 'https://horizon-testnet.stellar.org'
).toLowerCase();
const EVIDENCE_PATH = resolve(
  process.env.STELLARROUTE_SMOKE_EVIDENCE_PATH ??
    './tmp/live-swap-smoke-evidence.json',
);

const evidence = {
  date_utc: new Date().toISOString(),
  api_url: API_URL,
  network: NETWORK,
  passphrase: PASSPHRASE,
  horizon_url: HORIZON_URL,
  base: BASE,
  quote: QUOTE,
  amount: AMOUNT,
  sender: SENDER,
  mode: 'dry-run',
  steps: {},
  blockers: [],
  exit_ok: false,
};

function log(step, msg) {
  console.log(`[${step}] ${msg}`);
}

/**
 * Reject clearly mainnet-looking / public production API URLs.
 * Avoid false positives on localhost, testnet/staging/dev hosts, and tunnels.
 */
function rejectPublicApiUrl() {
  let url;
  try {
    url = new URL(API_URL);
  } catch {
    evidence.blockers.push('Invalid STELLARROUTE_API_URL');
    return false;
  }

  const host = url.hostname.toLowerCase();
  if (
    host === 'localhost' ||
    host === '127.0.0.1' ||
    host === '[::1]' ||
    host === '::1'
  ) {
    return true;
  }
  if (
    host.includes('testnet') ||
    host.includes('staging') ||
    host.includes('.dev') ||
    host.startsWith('dev.')
  ) {
    return true;
  }

  const bannedExact = new Set([
    'horizon.stellar.org',
    'api.stellar.org',
    'api.stellarroute.io',
    'api.stellarroute.app',
  ]);
  if (bannedExact.has(host)) {
    evidence.blockers.push(
      `Refusing public/production API host '${host}' — testnet only`,
    );
    return false;
  }
  if (
    (host.includes('mainnet') || host.includes('pubnet')) &&
    !host.includes('testnet')
  ) {
    evidence.blockers.push(
      `Refusing mainnet-looking API host '${host}' — testnet only`,
    );
    return false;
  }
  return true;
}

function rejectPublicNetwork() {
  const bannedNetwork = ['public', 'mainnet', 'pubnet'].includes(NETWORK);
  const bannedPassphrase =
    PASSPHRASE.toLowerCase().includes('public global') ||
    PASSPHRASE === 'Public Global Stellar Network ; September 2015';
  const bannedHorizon =
    HORIZON_URL.includes('horizon.stellar.org') &&
    !HORIZON_URL.includes('horizon-testnet');

  if (bannedNetwork || bannedPassphrase || bannedHorizon) {
    evidence.blockers.push(
      'Refusing public/mainnet network, passphrase, or Horizon URL — testnet only',
    );
    return false;
  }
  if (!rejectPublicApiUrl()) {
    return false;
  }
  return true;
}

async function httpJson(path, { method = 'GET', body } = {}) {
  const response = await fetch(`${API_URL}${path}`, {
    method,
    headers: {
      Accept: 'application/json',
      ...(body ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await response.text();
  let json;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = { raw: text };
  }
  return { status: response.status, json };
}

function unwrapData(json) {
  if (json && typeof json === 'object' && 'data' in json) return json.data;
  return json;
}

function looksLikePlaceholderXdr(xdr) {
  try {
    const decoded = Buffer.from(xdr, 'base64').toString('utf8');
    return decoded.startsWith('SR-PREPARE:');
  } catch {
    return false;
  }
}

function writeEvidence() {
  mkdirSync(dirname(EVIDENCE_PATH), { recursive: true });
  writeFileSync(EVIDENCE_PATH, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`\nEvidence written to ${EVIDENCE_PATH}`);
}

async function main() {
  if (process.env.STELLAR_SECRET_KEY || process.env.STELLARROUTE_SMOKE_FULL === '1') {
    evidence.blockers.push(
      'Secret-key / full-submit mode is not supported in this script. Use Freighter manually per docs/readiness/live-swap-testnet-checklist.md',
    );
    writeEvidence();
    process.exit(2);
  }

  if (!rejectPublicNetwork()) {
    writeEvidence();
    process.exit(2);
  }

  log('1.quote', `GET /api/v1/quote/${BASE}/${QUOTE}?amount=${AMOUNT}`);
  const quoteRes = await httpJson(
    `/api/v1/quote/${encodeURIComponent(BASE)}/${encodeURIComponent(QUOTE)}?amount=${AMOUNT}&quote_type=sell`,
  );
  evidence.steps.quote = {
    http_status: quoteRes.status,
    expires_at: unwrapData(quoteRes.json)?.expires_at ?? null,
    path_hops: unwrapData(quoteRes.json)?.path?.length ?? null,
  };
  if (quoteRes.status !== 200) {
    evidence.blockers.push(`quote returned HTTP ${quoteRes.status}`);
    writeEvidence();
    process.exit(1);
  }
  const quote = unwrapData(quoteRes.json);
  log('1.quote', `ok — hops=${quote.path?.length ?? 0}`);

  const hops = (quote.path ?? []).map((step) => {
    const from =
      step.from_asset?.asset_type === 'native'
        ? 'native'
        : `${step.from_asset?.asset_code}:${step.from_asset?.asset_issuer}`;
    const to =
      step.to_asset?.asset_type === 'native'
        ? 'native'
        : `${step.to_asset?.asset_code}:${step.to_asset?.asset_issuer}`;
    return {
      from_asset: from,
      to_asset: to,
      source: step.source,
      fee_bps: step.fee_bps,
      price: step.price,
    };
  });

  if (hops.length !== 1 || !/^sdex|^horizon/i.test(hops[0]?.source ?? '')) {
    evidence.blockers.push(
      'Dry-run requires a single classic SDEX/Horizon hop from quote.path',
    );
    writeEvidence();
    process.exit(1);
  }

  log('2.prepare', 'POST /api/v1/swap/prepare');
  const prepareRes = await httpJson('/api/v1/swap/prepare', {
    method: 'POST',
    body: {
      route: { hops },
      amount: AMOUNT,
      sender: SENDER,
      slippage_bps: 50,
    },
  });
  const prepared = unwrapData(prepareRes.json);
  const placeholder = prepared?.xdr_envelope
    ? looksLikePlaceholderXdr(prepared.xdr_envelope)
    : null;
  evidence.steps.prepare = {
    http_status: prepareRes.status,
    quote_id: prepared?.quote_id ?? null,
    expected_output: prepared?.expected_output ?? null,
    min_output: prepared?.min_output ?? null,
    expires_at: prepared?.expires_at ?? null,
    execution_mode: prepared?.execution_mode ?? null,
    network_passphrase: prepared?.network_passphrase ?? null,
    xdr_non_empty: Boolean(prepared?.xdr_envelope?.trim?.()),
    placeholder_xdr: placeholder,
  };

  if (prepareRes.status !== 200) {
    evidence.blockers.push(`prepare returned HTTP ${prepareRes.status}`);
    writeEvidence();
    process.exit(1);
  }
  if (prepared.execution_mode !== 'classic_path_payment') {
    evidence.blockers.push(
      `expected execution_mode classic_path_payment, got ${prepared.execution_mode}`,
    );
    writeEvidence();
    process.exit(1);
  }
  if (
    !prepared.network_passphrase ||
    prepared.network_passphrase.trim() !== PASSPHRASE
  ) {
    evidence.blockers.push(
      `prepare network_passphrase mismatch: got ${JSON.stringify(prepared.network_passphrase)}, expected ${JSON.stringify(PASSPHRASE)}`,
    );
    writeEvidence();
    process.exit(1);
  }
  if (!prepared.xdr_envelope?.trim()) {
    evidence.blockers.push('prepare returned empty xdr_envelope');
    writeEvidence();
    process.exit(1);
  }
  if (placeholder) {
    evidence.blockers.push(
      'prepare returned transitional placeholder XDR — classic PathPayment envelope required',
    );
    writeEvidence();
    process.exit(1);
  }

  evidence.exit_ok = true;
  writeEvidence();
  console.log('\nDry-run passed (quote + classic prepare).');
  console.log(
    'Manual Freighter submit/confirm: follow docs/readiness/live-swap-testnet-checklist.md steps 4–6.',
  );
}

main().catch((err) => {
  evidence.blockers.push(String(err?.stack ?? err));
  writeEvidence();
  console.error(err);
  process.exit(1);
});
