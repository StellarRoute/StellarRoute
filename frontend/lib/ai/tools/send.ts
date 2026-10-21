/**
 * Send Payment XDR Builder
 *
 * Constructs a classic Stellar Payment transaction from user-provided parameters
 * and returns the unsigned transaction envelope as a base64 XDR string,
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
  Memo,
  MemoType,
} from '@stellar/stellar-base';

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export type SendPaymentErrorCode =
  | 'invalid_destination'
  | 'invalid_asset'
  | 'invalid_amount'
  | 'invalid_memo'
  | 'account_fetch_failed'
  | 'build_failed';

export class SendPaymentError extends Error {
  constructor(
    public readonly code: SendPaymentErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'SendPaymentError';
  }
}

// ---------------------------------------------------------------------------
// Destination validation
// ---------------------------------------------------------------------------

/**
 * Validates a Stellar destination address (G-address).
 * Stellar public keys are 56 characters starting with 'G'.
 */
export function validateDestination(destination: string): void {
  if (!destination || typeof destination !== 'string') {
    throw new SendPaymentError('invalid_destination', 'Destination address is required');
  }

  const trimmed = destination.trim();

  // Stellar public keys: 56 chars, start with G, base32 alphabet
  const stellarAddressRegex = /^G[A-Z2-7]{55}$/;
  if (!stellarAddressRegex.test(trimmed)) {
    throw new SendPaymentError(
      'invalid_destination',
      `Invalid Stellar address: "${trimmed}". Must be a 56-character G-address.`,
    );
  }
}

// ---------------------------------------------------------------------------
// Asset parsing (reused from xdr-builder pattern)
// ---------------------------------------------------------------------------

/**
 * Convert a StellarRoute asset identifier string to a stellar-base `Asset`.
 *
 * Accepted formats:
 *   "native"            → Asset.native()  (XLM)
 *   "CODE:ISSUER"       → new Asset("CODE", "ISSUER")
 */
export function parseAsset(identifier: string): Asset {
  if (!identifier || typeof identifier !== 'string') {
    throw new SendPaymentError('invalid_asset', `Asset identifier must be a non-empty string, got: ${JSON.stringify(identifier)}`);
  }

  const trimmed = identifier.trim();

  if (trimmed.toLowerCase() === 'native') {
    return Asset.native();
  }

  const parts = trimmed.split(':');
  if (parts.length === 2) {
    const code = parts[0].trim();
    const issuer = parts[1].trim();

    if (!code || code.length > 12) {
      throw new SendPaymentError('invalid_asset', `Invalid asset code in "${trimmed}": "${code}"`);
    }
    if (!issuer || issuer.length < 56) {
      throw new SendPaymentError('invalid_asset', `Invalid issuer in "${trimmed}": "${issuer}"`);
    }
    return new Asset(code, issuer);
  }

  throw new SendPaymentError('invalid_asset', `Cannot parse asset identifier: "${trimmed}". Use "native" or "CODE:ISSUER" format.`);
}

// ---------------------------------------------------------------------------
// Amount validation
// ---------------------------------------------------------------------------

/**
 * Validates and normalizes the payment amount.
 * Returns the amount as a fixed 7-decimal string suitable for stellar-base.
 */
export function validateAmount(amount: string): string {
  if (!amount || typeof amount !== 'string') {
    throw new SendPaymentError('invalid_amount', 'Amount is required');
  }

  const parsed = parseFloat(amount);
  if (isNaN(parsed) || parsed <= 0) {
    throw new SendPaymentError('invalid_amount', `Amount must be a positive number, got: "${amount}"`);
  }

  // Stellar amounts are 7 decimal places fixed-point
  return parsed.toFixed(7);
}

// ---------------------------------------------------------------------------
// Memo handling
// ---------------------------------------------------------------------------

export interface MemoInput {
  type: 'text' | 'id' | 'hash' | 'return';
  value: string | number;
}

/**
 * Builds a stellar-base Memo from the input.
 */
export function buildMemo(memo: MemoInput): Memo {
  if (!memo || !memo.type) {
    return Memo.none();
  }

  switch (memo.type) {
    case 'text':
      if (typeof memo.value !== 'string') {
        throw new SendPaymentError('invalid_memo', 'Text memo value must be a string');
      }
      if (memo.value.length > 28) {
        throw new SendPaymentError('invalid_memo', 'Text memo exceeds 28-byte limit');
      }
      return new Memo(MemoType.memoText(), memo.value);

    case 'id':
      if (typeof memo.value !== 'number' && typeof memo.value !== 'string') {
        throw new SendPaymentError('invalid_memo', 'ID memo value must be a number or numeric string');
      }
      const id = typeof memo.value === 'string' ? parseInt(memo.value, 10) : memo.value;
      if (isNaN(id) || id < 0 || id > 0xffffffff) {
        throw new SendPaymentError('invalid_memo', 'ID memo must be a 32-bit unsigned integer');
      }
      return new Memo(MemoType.memoId(), id);

    case 'hash':
      if (typeof memo.value !== 'string') {
        throw new SendPaymentError('invalid_memo', 'Hash memo value must be a string');
      }
      // Expect 32-byte hex string (64 hex chars)
      if (memo.value.length !== 64 || !/^[0-9a-fA-F]{64}$/.test(memo.value)) {
        throw new SendPaymentError('invalid_memo', 'Hash memo must be a 32-byte hex string (64 hex characters)');
      }
      const hashBuffer = Buffer.from(memo.value, 'hex');
      return new Memo(MemoType.memoHash(), hashBuffer);

    case 'return':
      if (typeof memo.value !== 'string') {
        throw new SendPaymentError('invalid_memo', 'Return memo value must be a string');
      }
      // Expect 32-byte hex string (64 hex chars)
      if (memo.value.length !== 64 || !/^[0-9a-fA-F]{64}$/.test(memo.value)) {
        throw new SendPaymentError('invalid_memo', 'Return memo must be a 32-byte hex string (64 hex characters)');
      }
      const returnBuffer = Buffer.from(memo.value, 'hex');
      return new Memo(MemoType.memoReturn(), returnBuffer);

    default:
      return Memo.none();
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
    throw new SendPaymentError(
      'account_fetch_failed',
      `Network error fetching account from Horizon: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  if (!response.ok) {
    const status = response.status;
    if (status === 404) {
      throw new SendPaymentError(
        'account_fetch_failed',
        `Account not found on Horizon (${status}): ${walletAddress} — the account may not be funded on this network`,
      );
    }
    throw new SendPaymentError(
      'account_fetch_failed',
      `Horizon account lookup failed with HTTP ${status}`,
    );
  }

  let body: { sequence: string };
  try {
    body = await response.json() as { sequence: string };
  } catch {
    throw new SendPaymentError('account_fetch_failed', 'Could not parse Horizon account response');
  }

  if (!body.sequence) {
    throw new SendPaymentError('account_fetch_failed', 'Horizon account response missing sequence field');
  }

  return BigInt(body.sequence);
}

// ---------------------------------------------------------------------------
// XDR builder
// ---------------------------------------------------------------------------

export interface BuildSendPaymentXdrParams {
  /** Stellar public key (G-address) of the sender (source account) */
  sourceAddress: string;
  /** Stellar public key (G-address) of the recipient */
  destination: string;
  /** "native" or "CODE:ISSUER" */
  asset: string;
  /** Exact amount to send (as decimal string, e.g. "100.0000000") */
  amount: string;
  /** Optional memo */
  memo?: MemoInput;
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
 * Build an unsigned `Payment` transaction XDR for a classic Stellar send.
 *
 * Steps:
 *   1. Validate destination address
 *   2. Validate and parse asset
 *   3. Validate amount
 *   4. Build memo (if provided)
 *   5. Fetch account sequence from Horizon (or use `sequenceOverride` in tests)
 *   6. Build transaction with stellar-base `TransactionBuilder`
 *   7. Return unsigned base64 XDR envelope
 *
 * The returned XDR is then passed to the wallet for signing (e.g., Freighter, xBull)
 * and finally submitted to Horizon.
 *
 * This does NOT call `/api/v1/swap/prepare` — it builds a direct Payment operation.
 */
export async function buildSendPaymentXdr(params: BuildSendPaymentXdrParams): Promise<string> {
  // Step 1: Validate destination
  validateDestination(params.destination);

  // Step 2: Parse asset
  let sendAsset: Asset;
  try {
    sendAsset = parseAsset(params.asset);
  } catch (err) {
    if (err instanceof SendPaymentError) throw err;
    throw new SendPaymentError('invalid_asset', `Asset parsing failed: ${err instanceof Error ? err.message : String(err)}`);
  }

  // Step 3: Validate amount
  const sendAmount = validateAmount(params.amount);

  // Step 4: Build memo
  const memo = params.memo ? buildMemo(params.memo) : Memo.none();

  // Step 5: Fetch sequence (or use override for tests)
  let sequence: bigint;
  if (params.sequenceOverride !== undefined) {
    sequence = params.sequenceOverride;
  } else {
    sequence = await fetchAccountSequence(params.sourceAddress, params.horizonUrl);
  }

  // Step 6: Build transaction
  try {
    const sourceAccount = new Account(params.sourceAddress, sequence.toString());

    const timeout = params.timeoutSeconds ?? 30;

    const tx = new TransactionBuilder(sourceAccount, {
      fee: BASE_FEE,
      networkPassphrase: params.networkPassphrase,
    })
      .addOperation(
        Operation.payment({
          destination: params.destination,
          asset: sendAsset,
          amount: sendAmount,
        }),
      )
      .addMemo(memo)
      .setTimeout(timeout)
      .build();

    return tx.toEnvelope().toXDR('base64');
  } catch (err) {
    if (err instanceof SendPaymentError) throw err;
    throw new SendPaymentError(
      'build_failed',
      `Failed to build XDR: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}