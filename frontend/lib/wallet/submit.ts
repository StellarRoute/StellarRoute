/**
 * Swap prepare and Horizon transaction submission helpers.
 */

import { getApiBaseUrl } from '@/lib/network-endpoints';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';
import type { WalletNetwork } from './types';
import { getHorizonUrl, getNetworkPassphrase } from '@/lib/network-endpoints';

export { getHorizonUrl, getNetworkPassphrase };

export type HorizonSubmitErrorCode =
  | 'tx_bad_auth'
  | 'op_underfunded'
  | 'timeout'
  | 'horizon_error';

export class HorizonSubmitError extends Error {
  code: HorizonSubmitErrorCode;
  transactionCode?: string;
  operationCodes?: string[];

  constructor(
    code: HorizonSubmitErrorCode,
    message: string,
    details: { transactionCode?: string; operationCodes?: string[] } = {}
  ) {
    super(message);
    this.name = 'HorizonSubmitError';
    this.code = code;
    this.transactionCode = details.transactionCode;
    this.operationCodes = details.operationCodes;
  }
}

export interface HorizonSubmitResult {
  hash: string;
  ledger?: number;
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

function classifyHorizonError(body: HorizonErrorResponse): HorizonSubmitError {
  const txCode = body.extras?.result_codes?.transaction;
  const opCodes = body.extras?.result_codes?.operations ?? [];

  if (txCode === 'tx_bad_auth') {
    return new HorizonSubmitError(
      'tx_bad_auth',
      'Transaction signature was rejected by Stellar. Please reconnect your wallet and sign again.',
      { transactionCode: txCode, operationCodes: opCodes }
    );
  }

  if (opCodes.includes('op_underfunded')) {
    return new HorizonSubmitError(
      'op_underfunded',
      'Your wallet does not have enough spendable balance for this swap and network fees.',
      { transactionCode: txCode, operationCodes: opCodes }
    );
  }

  const ops = opCodes.length > 0 ? ` (${opCodes.join(', ')})` : '';
  const message = txCode
    ? `Transaction failed: ${txCode}${ops}`
    : (body.detail ?? body.title ?? 'Transaction submission failed');

  return new HorizonSubmitError('horizon_error', message, {
    transactionCode: txCode,
    operationCodes: opCodes,
  });
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException
    ? error.name === 'AbortError'
    : error instanceof Error && error.name === 'AbortError';
}

/**
 * Calls the StellarRoute prepare endpoint and returns the unsigned envelope XDR.
 */
export async function prepareSwapTransaction(
  params: TradeParams,
  network: WalletNetwork | null
): Promise<string> {
  const baseUrl = getApiBaseUrl(network);
  const response = await fetch(`${baseUrl}/api/v1/swap/prepare`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      from_asset: params.fromAsset,
      from_amount: params.fromAmount,
      to_asset: params.toAsset,
      min_received: params.minReceived,
      route_path: params.routePath,
      wallet_address: params.walletAddress,
    }),
  });

  if (!response.ok) {
    let message =
      'Unable to prepare this swap. Please refresh the quote and try again.';
    try {
      const body = (await response.json()) as {
        detail?: string;
        message?: string;
        error?: string;
      };
      message = body.detail ?? body.message ?? body.error ?? message;
    } catch {}
    throw new Error(message);
  }

  const body = (await response.json()) as {
    unsigned_xdr?: string;
    xdr?: string;
    transaction_xdr?: string;
  };
  const xdr = body.unsigned_xdr ?? body.xdr ?? body.transaction_xdr;
  if (!xdr) {
    throw new Error(
      'Swap prepare response did not include a transaction to sign.'
    );
  }
  return xdr;
}

/**
 * Submit a signed XDR envelope to Horizon and return the transaction hash.
 */
export async function submitToHorizon(
  signedXdr: string,
  network: WalletNetwork | null,
  opts: { timeoutMs?: number } = {}
): Promise<HorizonSubmitResult> {
  const horizonUrl = getHorizonUrl(network);
  const body = new URLSearchParams({ tx: signedXdr });
  const controller = new AbortController();
  const timeout = setTimeout(
    () => controller.abort(),
    opts.timeoutMs ?? 30_000
  );

  let response: Response;
  try {
    response = await fetch(`${horizonUrl}/transactions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
      signal: controller.signal,
    });
  } catch (error) {
    if (isAbortError(error)) {
      throw new HorizonSubmitError(
        'timeout',
        'Horizon did not respond in time. Please check the transaction status before retrying.'
      );
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }

  if (!response.ok) {
    try {
      throw classifyHorizonError(
        (await response.json()) as HorizonErrorResponse
      );
    } catch (error) {
      if (error instanceof HorizonSubmitError) throw error;
      throw new HorizonSubmitError(
        'horizon_error',
        `HTTP ${response.status}: Transaction submission failed`
      );
    }
  }

  const result = (await response.json()) as { hash: string; ledger?: number };
  return { hash: result.hash, ledger: result.ledger };
}
