import {
  signWithChainWallet,
  type SignTransactionRequest,
} from '@/lib/wallet/adapters';
import { submitToHorizon } from '@/lib/wallet/submit';
import type { WalletNetwork } from '@/lib/wallet/types';
import type { PreparedWalletPayload } from './types';
import { validatePreparedPayload } from './payload-validation';
import { executeEvmPreparedPayload } from './evm-execution';

export interface WalletExecutionResult {
  txHash: string;
  /** When false the hash is known but on-chain confirmation is still pending (EVM). */
  submissionReady: boolean;
}

export async function executePreparedPayload(input: {
  payload: PreparedWalletPayload;
  stellarAdapterId?: string;
  evmAdapterId?: string;
  walletNetwork?: WalletNetwork | null;
  expiresAtSec?: number;
  signal?: AbortSignal;
}): Promise<WalletExecutionResult> {
  if (input.payload.type === 'stellar_xdr') {
    return executeStellarPreparedPayload({
      payload: input.payload,
      stellarAdapterId: input.stellarAdapterId,
      walletNetwork: input.walletNetwork,
      expiresAtSec: input.expiresAtSec,
    });
  }
  if (!input.evmAdapterId) {
    throw new Error('Connect an EVM wallet on Sepolia to sign.');
  }
  const outcome = await executeEvmPreparedPayload({
    payload: input.payload,
    evmAdapterId: input.evmAdapterId,
    expiresAtSec: input.expiresAtSec,
    signal: input.signal,
  });
  return {
    txHash: outcome.txHash,
    submissionReady: outcome.status === 'confirmed',
  };
}

export async function executeStellarPreparedPayload(input: {
  payload: Extract<PreparedWalletPayload, { type: 'stellar_xdr' }>;
  stellarAdapterId?: string;
  walletNetwork?: WalletNetwork | null;
  expiresAtSec?: number;
}): Promise<WalletExecutionResult> {
  const validation = validatePreparedPayload(input.payload, {
    expiresAtSec: input.expiresAtSec,
  });
  if (!validation.ok) {
    const err = new Error(validation.message) as Error & { code: string };
    err.code = validation.code;
    throw err;
  }
  if (!input.stellarAdapterId) {
    throw new Error('Connect a Stellar wallet to sign.');
  }
  const signReq: SignTransactionRequest = {
    kind: 'stellar_xdr',
    xdr: input.payload.xdr_envelope,
    networkPassphrase: input.payload.network_passphrase,
  };
  const signed = await signWithChainWallet(input.stellarAdapterId, signReq);
  if (signed.kind !== 'stellar_xdr' || !signed.signedXdr?.trim()) {
    throw new Error('Wallet returned an empty signed envelope.');
  }
  try {
    const result = await submitToHorizon(
      signed.signedXdr,
      input.walletNetwork ?? 'testnet',
    );
    return { txHash: result.hash, submissionReady: true };
  } catch (submitErr) {
    const recovered = await recoverHorizonByHash(
      signed.signedXdr,
      input.payload.network_passphrase,
      input.walletNetwork ?? 'testnet',
    );
    if (recovered) return { txHash: recovered, submissionReady: true };
    throw submitErr;
  }
}

export async function reconcileEvmTransactionHash(input: {
  txHash: string;
  signal?: AbortSignal;
}): Promise<WalletExecutionResult> {
  const { pollEvmTransactionReceipt } = await import('./evm-receipt');
  const status = await pollEvmTransactionReceipt(input.txHash, {
    signal: input.signal,
  });
  if (status === 'reverted') {
    const err = new Error('EVM transaction reverted on-chain.') as Error & {
      code: string;
    };
    err.code = 'nonretryable';
    throw err;
  }
  return {
    txHash: input.txHash,
    submissionReady: status === 'success',
  };
}

async function recoverHorizonByHash(
  signedXdr: string,
  networkPassphrase: string,
  network: WalletNetwork | null,
): Promise<string | null> {
  try {
    const { TransactionBuilder } = await import('@stellar/stellar-base');
    const tx = TransactionBuilder.fromXDR(signedXdr, networkPassphrase);
    const hash = tx.hash().toString('hex');
    const { getHorizonUrl } = await import('@/lib/wallet/submit');
    const horizonUrl = getHorizonUrl(network);
    if (!horizonUrl.includes('testnet')) {
      return null;
    }
    const response = await fetch(
      `${horizonUrl}/transactions/${encodeURIComponent(hash)}`,
    );
    if (response.ok) {
      const body = (await response.json()) as { hash?: string };
      return body.hash ?? hash;
    }
  } catch {
    // recovery failed — do not resubmit blindly
  }
  return null;
}
