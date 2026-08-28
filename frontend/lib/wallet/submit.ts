/**
 * Horizon transaction submission helpers.
 *
 * Builds a minimal Stellar transaction from the quote/route metadata,
 * submits the signed XDR to Horizon, and returns the transaction hash.
 */

import type { WalletNetwork } from './types';
import {
  getHorizonUrl,
  getNetworkPassphrase,
} from '@/lib/network-endpoints';

export { getHorizonUrl, getNetworkPassphrase };

export interface HorizonSubmitResult {
  hash: string;
  ledger?: number;
}

export type HorizonSubmitErrorCode =
  | 'tx_bad_auth'
  | 'tx_bad_seq'
  | 'op_underfunded'
  | 'timeout'
  | 'horizon_error';

export class HorizonSubmitError extends Error {
  readonly code: HorizonSubmitErrorCode;
  readonly transactionCode?: string;
  readonly operationCodes?: string[];
  readonly status?: number;

  constructor(
    message: string,
    {
      code,
      transactionCode,
      operationCodes,
      status,
    }: {
      code: HorizonSubmitErrorCode;
      transactionCode?: string;
      operationCodes?: string[];
      status?: number;
    },
  ) {
    super(message);
    this.name = 'HorizonSubmitError';
    this.code = code;
    this.transactionCode = transactionCode;
    this.operationCodes = operationCodes;
    this.status = status;
  }
}

interface HorizonErrorExtras {
  result_codes?: {
    transaction?: string;
    operations?: string[];
  };
}

interface HorizonErrorResponse {
  extras?: HorizonErrorExtras;
  title?: string;
  detail?: string;
}

function classifyHorizonError(
  txCode?: string,
  opCodes: string[] = [],
): HorizonSubmitErrorCode {
  if (txCode === 'tx_bad_auth') return 'tx_bad_auth';
  if (txCode === 'tx_bad_seq') return 'tx_bad_seq';
  if (opCodes.includes('op_underfunded')) return 'op_underfunded';
  return 'horizon_error';
}

function extractHorizonError(
  body: HorizonErrorResponse,
  status: number,
): HorizonSubmitError {
  const txCode = body.extras?.result_codes?.transaction;
  const opCodes = body.extras?.result_codes?.operations ?? [];
  const code = classifyHorizonError(txCode, opCodes);
  if (txCode) {
    const ops = opCodes.join(', ');
    const message = ops
      ? `Transaction failed: ${txCode} (${ops})`
      : `Transaction failed: ${txCode}`;
    return new HorizonSubmitError(message, {
      code,
      transactionCode: txCode,
      operationCodes: opCodes,
      status,
    });
  }
  return new HorizonSubmitError(
    body.detail ?? body.title ?? 'Transaction submission failed',
    { code, status },
  );
}

/**
 * Submit a signed XDR envelope to Horizon and return the transaction hash.
 *
 * @param signedXdr  Base64-encoded signed transaction envelope XDR
 * @param network    Wallet / app network context (testnet | mainnet | ...)
 */
export async function submitToHorizon(
  signedXdr: string,
  network: WalletNetwork | null,
): Promise<HorizonSubmitResult> {
  const horizonUrl = getHorizonUrl(network);
  const body = new URLSearchParams({ tx: signedXdr });

  let response: Response;
  try {
    response = await fetch(`${horizonUrl}/transactions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : '';
    const isTimeout = message.toLowerCase().includes('timeout');
    throw new HorizonSubmitError(
      isTimeout
        ? 'Transaction submission timed out. Please check your wallet activity before trying again.'
        : 'Unable to reach Horizon. Please check your connection and try again.',
      { code: isTimeout ? 'timeout' : 'horizon_error' },
    );
  }

  if (!response.ok) {
    let errorBody: HorizonErrorResponse;
    try {
      errorBody = (await response.json()) as HorizonErrorResponse;
    } catch {
      throw new HorizonSubmitError(
        `HTTP ${response.status}: Transaction submission failed`,
        { code: 'horizon_error', status: response.status },
      );
    }
    throw extractHorizonError(errorBody, response.status);
  }

  const result = await response.json() as { hash: string; ledger?: number };
  return { hash: result.hash, ledger: result.ledger };
}
