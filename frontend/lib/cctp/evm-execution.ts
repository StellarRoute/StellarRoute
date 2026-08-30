import {
  getAdapter,
  sendWithChainWallet,
  type AdapterNetworkId,
  type SendTransactionRequest,
} from '@/lib/wallet/adapters';
import { WalletAdapterError } from '@/lib/wallet/adapters';
import { caip2ToChainIdHex } from '@/lib/wallet/adapters/evm/networks';
import { assertSepoliaCaip, caip2FromChainIdHex } from './caip-evm';
import {
  pollEvmTransactionReceipt,
  type EvmReceiptPollDeps,
  type EvmReceiptStatus,
  DEFAULT_RECEIPT_TIMEOUT_MS,
} from './evm-receipt';
import type { PreparedWalletPayload } from './types';
import { validatePreparedPayload } from './payload-validation';

export const MAX_EVM_CALLDATA_BYTES = 24_576;

export type EvmExecutionOutcome =
  | { status: 'confirmed'; txHash: string }
  | { status: 'pending'; txHash: string }
  | { status: 'reverted'; txHash: string };

export type EvmSendDeps = {
  sendTransaction: (
    adapterId: string,
    request: SendTransactionRequest,
  ) => Promise<{ kind: 'evm_transaction'; hash: string }>;
  switchNetwork?: (
    adapterId: string,
    network: AdapterNetworkId,
  ) => Promise<void>;
  readChainIdHex?: (adapterId: string) => Promise<string | null>;
  waitForReceipt?: (
    txHash: string,
    opts: { signal?: AbortSignal; timeoutMs?: number },
  ) => Promise<EvmReceiptStatus>;
};

const defaultDeps: EvmSendDeps = {
  sendTransaction: async (adapterId, request) => {
    const result = await sendWithChainWallet(adapterId, request);
    if (result.kind !== 'evm_transaction' || !result.hash) {
      throw new Error('EVM wallet did not return a transaction hash.');
    }
    return { kind: 'evm_transaction', hash: result.hash };
  },
  switchNetwork: async (adapterId, network) => {
    const adapter = getAdapter(adapterId);
    if (!adapter?.switchNetwork) {
      throw new WalletAdapterError(
        'Wallet cannot switch networks automatically.',
        'unsupported_capability',
        adapterId,
      );
    }
    await adapter.switchNetwork(network);
  },
  readChainIdHex: async (adapterId) => {
    const adapter = getAdapter(adapterId);
    if (!adapter) return null;
    const info = await adapter.getNetwork();
    return caip2ToChainIdHex(info.network);
  },
  waitForReceipt: (txHash, opts) =>
    pollEvmTransactionReceipt(txHash, {
      signal: opts.signal,
      timeoutMs: opts.timeoutMs ?? DEFAULT_RECEIPT_TIMEOUT_MS,
    }),
};

export async function executeEvmPreparedPayload(input: {
  payload: Extract<PreparedWalletPayload, { type: 'evm_transaction' }>;
  evmAdapterId: string;
  expiresAtSec?: number;
  deps?: Partial<EvmSendDeps> & { receiptDeps?: EvmReceiptPollDeps };
  signal?: AbortSignal;
  receiptTimeoutMs?: number;
}): Promise<EvmExecutionOutcome> {
  const deps = { ...defaultDeps, ...input.deps };
  const validation = validatePreparedPayload(input.payload, {
    expiresAtSec: input.expiresAtSec,
  });
  if (!validation.ok) {
    const err = new Error(validation.message) as Error & { code: string };
    err.code = validation.code;
    throw err;
  }

  const parsed = assertSepoliaCaip(input.payload.chain_id);
  if (!parsed.ok) {
    const err = new Error(parsed.message) as Error & { code: string };
    err.code = parsed.code;
    throw err;
  }

  const expectedNetwork = caip2FromChainIdHex(parsed.chainIdHex);
  const currentHex = await deps.readChainIdHex!(input.evmAdapterId);
  if (currentHex?.toLowerCase() !== parsed.chainIdHex.toLowerCase()) {
    try {
      await deps.switchNetwork!(input.evmAdapterId, expectedNetwork);
    } catch (switchErr) {
      if (isUserRejected(switchErr)) {
        const err = new Error('Network switch declined in wallet.') as Error & {
          code: string;
        };
        err.code = 'user_rejected';
        throw err;
      }
      throw switchErr;
    }
    const afterHex = await deps.readChainIdHex!(input.evmAdapterId);
    if (afterHex?.toLowerCase() !== parsed.chainIdHex.toLowerCase()) {
      throw new WalletAdapterError(
        'Wallet network does not match Sepolia after switch.',
        'network_mismatch',
        input.evmAdapterId,
      );
    }
  }

  const valueHex = normalizeEvmValueHex(input.payload.value);
  const sendReq: SendTransactionRequest = {
    kind: 'evm_transaction',
    transaction: {
      chainId: parsed.chainIdHex,
      to: input.payload.to,
      data: input.payload.data,
      value: valueHex,
      gas: input.payload.gas,
      gasPrice: input.payload.gas_price,
      maxFeePerGas: input.payload.max_fee_per_gas,
      maxPriorityFeePerGas: input.payload.max_priority_fee_per_gas,
    },
  };

  const sent = await deps.sendTransaction(input.evmAdapterId, sendReq);
  const receiptStatus = await deps.waitForReceipt!(sent.hash, {
    signal: input.signal,
    timeoutMs: input.receiptTimeoutMs ?? DEFAULT_RECEIPT_TIMEOUT_MS,
  });

  if (receiptStatus === 'reverted') {
    const err = new Error('EVM transaction reverted on-chain.') as Error & {
      code: string;
    };
    err.code = 'nonretryable';
    throw err;
  }

  if (receiptStatus === 'pending') {
    return { status: 'pending', txHash: sent.hash };
  }

  return { status: 'confirmed', txHash: sent.hash };
}

function normalizeEvmValueHex(value: string): string {
  if (value.startsWith('0x')) return value;
  const asBig = BigInt(value);
  return `0x${asBig.toString(16)}`;
}

function isUserRejected(err: unknown): boolean {
  if (err instanceof WalletAdapterError && err.code === 'user_rejected') {
    return true;
  }
  if (err instanceof Error && /reject|denied|cancel/i.test(err.message)) {
    return true;
  }
  return false;
}
