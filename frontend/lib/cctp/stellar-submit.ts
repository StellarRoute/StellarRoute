import { StellarRouteApiError } from '@/lib/api/client';
import type { CctpApiClient } from './client';
import type { CctpCallOptions } from './types';
import type { WalletNetwork } from '@/lib/wallet/types';

const SUBMIT_BURN_MAX_ATTEMPTS = 6;
const SUBMIT_BURN_BASE_MS = 400;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function isSubmitBurnRetryable(err: unknown): boolean {
  if (err instanceof StellarRouteApiError) {
    return err.status === 503 || err.code === 'dependency_unavailable';
  }
  return false;
}

/** Wait until Horizon can see the tx (Soroban RPC may lag behind). */
export async function waitForHorizonTransaction(
  txHash: string,
  network: WalletNetwork | null = 'testnet',
  options?: { signal?: AbortSignal; maxAttempts?: number },
): Promise<void> {
  const { getHorizonUrl } = await import('@/lib/wallet/submit');
  const horizonUrl = getHorizonUrl(network);
  const maxAttempts = options?.maxAttempts ?? 12;
  const normalized = txHash.trim().toLowerCase().replace(/^0x/, '');

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    if (options?.signal?.aborted) {
      throw new StellarRouteApiError(0, 'network_error', 'Submission cancelled');
    }
    try {
      const response = await fetch(
        `${horizonUrl}/transactions/${encodeURIComponent(normalized)}`,
        { signal: options?.signal },
      );
      if (response.ok) {
        const body = (await response.json()) as { successful?: boolean };
        if (body.successful === false) {
          throw new StellarRouteApiError(
            400,
            'validation_error',
            'Stellar transaction failed on-chain.',
          );
        }
        return;
      }
      if (response.status !== 404) {
        break;
      }
    } catch (err) {
      if (err instanceof StellarRouteApiError) throw err;
      if (options?.signal?.aborted) {
        throw new StellarRouteApiError(0, 'network_error', 'Submission cancelled');
      }
    }
    await sleep(350 + attempt * 200);
  }
}

export async function submitBurnWithVerificationRetry(
  client: CctpApiClient,
  transferId: string,
  txHash: string,
  options?: CctpCallOptions,
  network: WalletNetwork | null = 'testnet',
): Promise<void> {
  await waitForHorizonTransaction(txHash, network, { signal: options?.signal });

  let lastErr: unknown;
  for (let attempt = 0; attempt < SUBMIT_BURN_MAX_ATTEMPTS; attempt++) {
    if (options?.signal?.aborted) {
      throw new StellarRouteApiError(0, 'network_error', 'Submission cancelled');
    }
    try {
      await client.submitBurn(transferId, { tx_hash: txHash }, options);
      return;
    } catch (err) {
      lastErr = err;
      if (isSubmitBurnRetryable(err) && attempt + 1 < SUBMIT_BURN_MAX_ATTEMPTS) {
        await sleep(SUBMIT_BURN_BASE_MS * Math.pow(2, attempt));
        continue;
      }
      throw err;
    }
  }
  throw lastErr;
}
