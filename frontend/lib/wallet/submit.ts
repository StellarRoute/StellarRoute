/**
 * Horizon transaction submission helpers.
 *
 * Builds a minimal Stellar transaction from the quote/route metadata,
 * submits the signed XDR to Horizon, and returns the transaction hash.
 */

import type { WalletNetwork } from './types';

const HORIZON_URLS: Record<string, string> = {
  testnet: 'https://horizon-testnet.stellar.org',
  mainnet: 'https://horizon.stellar.org',
};

const NETWORK_PASSPHRASES: Record<string, string> = {
  testnet: 'Test SDF Network ; September 2015',
  mainnet: 'Public Global Stellar Network ; September 2015',
  futurenet: 'Test SDF Future Network ; October 2022',
};

export function getHorizonUrl(network: WalletNetwork | null): string {
  const defaultNetwork = process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';
  const key = String(network ?? defaultNetwork).toLowerCase();
  if (key === defaultNetwork.toLowerCase() && process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL) {
    return process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL;
  }
  return HORIZON_URLS[key] ?? HORIZON_URLS[defaultNetwork] ?? HORIZON_URLS.testnet;
}

export function getNetworkPassphrase(network: WalletNetwork | null): string {
  const defaultNetwork = process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';
  const key = String(network ?? defaultNetwork).toLowerCase();
  return NETWORK_PASSPHRASES[key] ?? NETWORK_PASSPHRASES[defaultNetwork] ?? NETWORK_PASSPHRASES.testnet;
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

function extractHorizonError(body: HorizonErrorResponse): string {
  const txCode = body.extras?.result_codes?.transaction;
  const opCodes = body.extras?.result_codes?.operations;
  if (txCode) {
    const ops = opCodes?.join(', ');
    return ops ? `Transaction failed: ${txCode} (${ops})` : `Transaction failed: ${txCode}`;
  }
  return body.detail ?? body.title ?? 'Transaction submission failed';
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

  const response = await fetch(`${horizonUrl}/transactions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  });

  if (!response.ok) {
    let errorMessage: string;
    try {
      const errorBody = (await response.json()) as HorizonErrorResponse;
      errorMessage = extractHorizonError(errorBody);
    } catch {
      errorMessage = `HTTP ${response.status}: Transaction submission failed`;
    }
    throw new Error(errorMessage);
  }

  const result = await response.json() as { hash: string; ledger?: number };
  return { hash: result.hash, ledger: result.ledger };
}
