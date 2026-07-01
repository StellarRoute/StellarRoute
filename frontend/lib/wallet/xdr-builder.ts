/**
 * Stellar XDR Builder
 *
 * Constructs a PathPaymentStrictSend transaction from StellarRoute quote
 * data and returns the unsigned transaction envelope as a base64 XDR string,
 * ready to be passed to the wallet for signing.
 *
 * This module has no React dependencies and is purely functional so it can be
 * imported in unit tests without a browser environment.
 */

import {
  Asset,
  TransactionBuilder,
  Operation,
  Account,
  Networks,
  BASE_FEE,
} from '@stellar/stellar-base';
import type { PathStep } from '@/types';

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export type XdrBuildErrorCode =
  | 'invalid_asset'
  | 'invalid_quote_shape'
  | 'account_fetch_failed'
  | 'build_failed';

export class XdrBuildError extends Error {
  constructor(
    public readonly code: XdrBuildErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'XdrBuildError';
  }
}

// ---------------------------------------------------------------------------
// Asset parsing
// ---------------------------------------------------------------------------

/**
 * Convert a StellarRoute asset identifier string to a stellar-base `Asset`.
 *
 * Accepted formats:
 *   "native"            → Asset.native()  (XLM)
 *   "CODE"              → new Asset("CODE", undefined)  — issuer-less (will
 *                         resolve via path payment network routing)
 *   "CODE:ISSUER"       → new Asset("CODE", "ISSUER")
 */
export function parseAsset(identifier: string): Asset {
  if (!identifier || typeof identifier !== 'string') {
    throw new XdrBuildError('invalid_asset', `Asset identifier must be a non-empty string, got: ${JSON.stringify(identifier)}`);
  }

  const trimmed = identifier.trim();

  if (trimmed.toLowerCase() === 'native') {
    return Asset.native();
  }

  const parts = trimmed.split(':');
  if (parts.length === 1) {
    const code = parts[0].trim();
    if (!code || code.length > 12) {
      throw new XdrBuildError('invalid_asset', `Invalid asset code: "${code}"`);
    }
    // Issuer-less assets are not valid on Stellar — callers must always provide
    // CODE:ISSUER for non-native assets. Throw a clear error.
    throw new XdrBuildError(
      'invalid_asset',
      `Non-native asset "${code}" requires an issuer in the format CODE:ISSUER`,
    );
  }

  if (parts.length === 2) {
    const code = parts[0].trim();
    const issuer = parts[1].trim();

    if (!code || code.length > 12) {
      throw new XdrBuildError('invalid_asset', `Invalid asset code in "${trimmed}": "${code}"`);
    }
    if (!issuer || issuer.length < 56) {
      throw new XdrBuildError('invalid_asset', `Invalid issuer in "${trimmed}": "${issuer}"`);
    }
    return new Asset(code, issuer);
  }

  throw new XdrBuildError('invalid_asset', `Cannot parse asset identifier: "${trimmed}"`);
}

// ---------------------------------------------------------------------------
// Quote shape validation
// ---------------------------------------------------------------------------

export interface QuoteShapeParams {
  walletAddress: string;
  fromAsset: string;
  fromAmount: string;
  toAsset: string;
  minReceived: string;
}

/**
 * Validate the trade parameters before any network calls or wallet prompts.
 * Throws `XdrBuildError` with code `invalid_quote_shape` on bad input.
 */
export function validateQuoteShape(params: QuoteShapeParams): void {
  if (!params.walletAddress || params.walletAddress.trim().length < 56) {
    throw new XdrBuildError(
      'invalid_quote_shape',
      'walletAddress is missing or invalid — cannot build transaction without a funded source account',
    );
  }
  if (!params.fromAsset || params.fromAsset.trim() === '') {
    throw new XdrBuildError('invalid_quote_shape', 'fromAsset is required');
  }
  if (!params.toAsset || params.toAsset.trim() === '') {
    throw new XdrBuildError('invalid_quote_shape', 'toAsset is required');
  }

  const amount = parseFloat(params.fromAmount);
  if (!params.fromAmount || isNaN(amount) || amount <= 0) {
    throw new XdrBuildError(
      'invalid_quote_shape',
      `fromAmount must be a positive number, got: "${params.fromAmount}"`,
    );
  }

  const minRec = parseFloat(params.minReceived);
  if (!params.minReceived || isNaN(minRec) || minRec < 0) {
    throw new XdrBuildError(
      'invalid_quote_shape',
      `minReceived must be a non-negative number, got: "${params.minReceived}"`,
    );
  }
}

// ---------------------------------------------------------------------------
// Account sequence lookup
// ---------------------------------------------------------------------------

/**
 * Fetch the current sequence number for `walletAddress` from Horizon.
 * Returns the sequence as a `bigint` (required by stellar-base `Account`).
 */
export async function fetchAccountSequence(
  walletAddress: string,
  horizonUrl: string,
): Promise<bigint> {
  const url = `${horizonUrl.replace(/\/$/, '')}/accounts/${encodeURIComponent(walletAddress)}`;

  let response: Response;
  try {
    response = await fetch(url, {
      headers: { Accept: 'application/json' },
    });
  } catch (err) {
    throw new XdrBuildError(
      'account_fetch_failed',
      `Network error fetching account from Horizon: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  if (!response.ok) {
    const status = response.status;
    if (status === 404) {
      throw new XdrBuildError(
        'account_fetch_failed',
        `Account not found on Horizon (${status}): ${walletAddress} — the account may not be funded on this network`,
      );
    }
    throw new XdrBuildError(
      'account_fetch_failed',
      `Horizon account lookup failed with HTTP ${status}`,
    );
  }

  let body: { sequence: string };
  try {
    body = await response.json() as { sequence: string };
  } catch {
    throw new XdrBuildError('account_fetch_failed', 'Could not parse Horizon account response');
  }

  if (!body.sequence) {
    throw new XdrBuildError('account_fetch_failed', 'Horizon account response missing sequence field');
  }

  return BigInt(body.sequence);
}

// ---------------------------------------------------------------------------
// XDR builder
// ---------------------------------------------------------------------------

export interface BuildXdrParams {
  /** Stellar public key (G-address) of the swap sender */
  walletAddress: string;
  /** "native" or "CODE:ISSUER" */
  fromAsset: string;
  /** Exact amount of fromAsset to send (as decimal string, e.g. "100.0000000") */
  fromAmount: string;
  /** "native" or "CODE:ISSUER" */
  toAsset: string;
  /** Minimum destination amount after slippage (as decimal string) */
  minReceived: string;
  /** Intermediate hops from the quote path */
  routePath: PathStep[];
  /** Stellar network passphrase */
  networkPassphrase: string;
  /** Horizon base URL used to fetch the account sequence */
  horizonUrl: string;
  /** Transaction timeout in seconds (default: 30) */
  timeoutSeconds?: number;
  /**
   * Override the account sequence — used in tests to avoid hitting Horizon.
   * When provided, `fetchAccountSequence` is skipped entirely.
   */
  sequenceOverride?: bigint;
}

/**
 * Build an unsigned `PathPaymentStrictSend` transaction XDR from quote data.
 *
 * Steps:
 *   1. `validateQuoteShape` — throws before any I/O on malformed inputs
 *   2. Fetch account sequence from Horizon (or use `sequenceOverride` in tests)
 *   3. Build transaction with stellar-base `TransactionBuilder`
 *   4. Return unsigned base64 XDR envelope
 *
 * The returned XDR is then passed to `signTransactionWithWallet` (Freighter/xBull)
 * and finally to `submitToHorizon`.
 */
export async function buildPathPaymentXdr(params: BuildXdrParams): Promise<string> {
  // Step 1: validate before any I/O
  validateQuoteShape({
    walletAddress: params.walletAddress,
    fromAsset: params.fromAsset,
    fromAmount: params.fromAmount,
    toAsset: params.toAsset,
    minReceived: params.minReceived,
  });

  // Step 2: fetch sequence (or use override for tests)
  let sequence: bigint;
  if (params.sequenceOverride !== undefined) {
    sequence = params.sequenceOverride;
  } else {
    sequence = await fetchAccountSequence(params.walletAddress, params.horizonUrl);
  }

  // Step 3: build transaction
  try {
    const sourceAccount = new Account(params.walletAddress, sequence.toString());

    const sendAsset = parseAsset(params.fromAsset);
    const destAsset = parseAsset(params.toAsset);

    // Intermediate path assets: the assets at the output of each hop except
    // the last (which is destAsset). stellar-base path payment `path` contains
    // only intermediate assets, not send or dest.
    const pathAssets: Asset[] = params.routePath
      .slice(0, -1)  // drop last hop — its to_asset is the destAsset
      .map((step) => {
        const toId = step.to_asset;
        const type = toId.asset_type;
        if (type === 'native') return Asset.native();
        const code = toId.asset_code ?? '';
        const issuer = toId.asset_issuer ?? '';
        if (!code || !issuer) return Asset.native(); // fallback for malformed
        return new Asset(code, issuer);
      });

    const timeout = params.timeoutSeconds ?? 30;

    const tx = new TransactionBuilder(sourceAccount, {
      fee: BASE_FEE,
      networkPassphrase: params.networkPassphrase,
    })
      .addOperation(
        Operation.pathPaymentStrictSend({
          sendAsset,
          sendAmount: parseFloat(params.fromAmount).toFixed(7),
          destination: params.walletAddress, // self-trade — the dApp submits on behalf of user
          destAsset,
          destMin: parseFloat(params.minReceived).toFixed(7),
          path: pathAssets,
        }),
      )
      .setTimeout(timeout)
      .build();

    return tx.toEnvelope().toXDR('base64');
  } catch (err) {
    if (err instanceof XdrBuildError) throw err;
    throw new XdrBuildError(
      'build_failed',
      `Failed to build XDR: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}
