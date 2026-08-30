/**
 * Explicit chain wallet session state helpers (non-React).
 * Used by `useChainWallet` and tests — does not touch Stellar WalletProvider.
 */

import { WalletAdapterError } from './errors';
import { getAdapter, listAvailableChainWallets } from './registry';
import type {
  AdapterNetworkId,
  AvailableChainWallet,
  ChainFamily,
  ChainNetworkInfo,
  ChainWalletSession,
  ExecutionSupport,
  SendTransactionRequest,
  SendTransactionResult,
  SignMessageRequest,
  SignTransactionRequest,
  SignedMessageResult,
  SignedTransactionResult,
} from './types';

export type ChainWalletState = {
  session: ChainWalletSession | null;
  expectedNetwork: AdapterNetworkId | null;
  networkInfo: ChainNetworkInfo | null;
  networkMismatch: boolean;
  availableWallets: AvailableChainWallet[];
  isLoading: boolean;
  error: { message: string; code?: string } | null;
};

export function createEmptyChainWalletState(
  expectedNetwork: AdapterNetworkId | null = null
): ChainWalletState {
  return {
    session: null,
    expectedNetwork,
    networkInfo: null,
    networkMismatch: false,
    availableWallets: [],
    isLoading: false,
    error: null,
  };
}

export async function refreshAvailableWallets(
  chainFamily?: ChainFamily
): Promise<AvailableChainWallet[]> {
  return listAvailableChainWallets(chainFamily);
}

export async function connectChainWallet(
  adapterId: string,
  expectedNetwork?: AdapterNetworkId
): Promise<{
  session: ChainWalletSession;
  networkInfo: ChainNetworkInfo;
  networkMismatch: boolean;
}> {
  const adapter = getAdapter(adapterId);
  if (!adapter) {
    throw new WalletAdapterError(
      `Unknown wallet adapter: ${adapterId}`,
      'not_installed',
      adapterId
    );
  }

  const session = await adapter.connect(expectedNetwork);
  const networkInfo = await adapter.getNetwork(expectedNetwork);
  const networkMismatch = expectedNetwork
    ? !networkInfo.matchesExpected
    : false;

  return { session, networkInfo, networkMismatch };
}

export async function disconnectChainWallet(
  adapterId: string | null | undefined
): Promise<void> {
  if (!adapterId) return;
  const adapter = getAdapter(adapterId);
  if (!adapter) return;
  await adapter.disconnect();
}

export async function signWithChainWallet(
  adapterId: string,
  request: SignTransactionRequest
): Promise<SignedTransactionResult> {
  const adapter = getAdapter(adapterId);
  if (!adapter) {
    throw new WalletAdapterError(
      `Unknown wallet adapter: ${adapterId}`,
      'not_installed',
      adapterId
    );
  }
  return adapter.signTransaction(request);
}

export async function signMessageWithChainWallet(
  adapterId: string,
  request: SignMessageRequest
): Promise<SignedMessageResult> {
  const adapter = getAdapter(adapterId);
  if (!adapter) {
    throw new WalletAdapterError(
      `Unknown wallet adapter: ${adapterId}`,
      'not_installed',
      adapterId
    );
  }
  return adapter.signMessage(request);
}

export async function sendWithChainWallet(
  adapterId: string,
  request: SendTransactionRequest
): Promise<SendTransactionResult> {
  const adapter = getAdapter(adapterId);
  if (!adapter) {
    throw new WalletAdapterError(
      `Unknown wallet adapter: ${adapterId}`,
      'not_installed',
      adapterId
    );
  }
  if (typeof adapter.sendTransaction !== 'function') {
    throw new WalletAdapterError(
      `Adapter ${adapterId} does not support sendTransaction`,
      'unsupported_capability',
      adapterId
    );
  }
  return adapter.sendTransaction(request);
}

export function getChainExecutionSupport(
  adapterId: string,
  routeHint?: {
    sourceChain?: ChainFamily;
    destinationChain?: ChainFamily;
  }
): ExecutionSupport {
  const adapter = getAdapter(adapterId);
  if (!adapter) {
    return {
      kind: 'unsupported',
      code: 'not_connected',
      message: `Unknown wallet adapter: ${adapterId}`,
      resolution: 'Choose a registered wallet adapter',
    };
  }
  return adapter.getExecutionSupport(routeHint);
}
