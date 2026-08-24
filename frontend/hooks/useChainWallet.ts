'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  connectChainWallet,
  createEmptyChainWalletState,
  disconnectChainWallet,
  refreshAvailableWallets,
  resolveExecutionSupport,
  sendWithChainWallet,
  signMessageWithChainWallet,
  signWithChainWallet,
  type AdapterNetworkId,
  type ChainFamily,
  type ChainWalletState,
  type SendTransactionRequest,
  type SignMessageRequest,
  type SignTransactionRequest,
} from '@/lib/wallet/adapters';

/**
 * Chain-aware wallet hook for registered multi-chain adapters
 * (EVM, Solana, Bitcoin, TRON, and thin Stellar wrappers).
 *
 * Intentionally separate from the Stellar `WalletProvider` / `useWallet`
 * path so Freighter/xBull/Albedo/LOBSTR swap UX stays unchanged.
 */
export function useChainWallet(options?: {
  chainFamily?: ChainFamily;
  expectedNetwork?: AdapterNetworkId;
}) {
  const chainFamily = options?.chainFamily;
  const expectedNetwork = options?.expectedNetwork;

  const [state, setState] = useState<ChainWalletState>(() =>
    createEmptyChainWalletState(expectedNetwork ?? null)
  );

  const refreshWallets = useCallback(async () => {
    const availableWallets = await refreshAvailableWallets(chainFamily);
    setState((prev) => ({ ...prev, availableWallets }));
  }, [chainFamily]);

  useEffect(() => {
    void refreshWallets();
  }, [refreshWallets]);

  useEffect(() => {
    setState((prev) => ({
      ...prev,
      expectedNetwork: expectedNetwork ?? prev.expectedNetwork,
    }));
  }, [expectedNetwork]);

  const connect = useCallback(
    async (adapterId: string, networkOverride?: AdapterNetworkId) => {
      setState((prev) => ({ ...prev, isLoading: true, error: null }));
      try {
        const target = networkOverride ?? expectedNetwork ?? undefined;
        const result = await connectChainWallet(adapterId, target);
        setState((prev) => ({
          ...prev,
          session: result.session,
          networkInfo: result.networkInfo,
          networkMismatch: result.networkMismatch,
          expectedNetwork: target ?? prev.expectedNetwork,
          isLoading: false,
          error: null,
        }));
        return result.session;
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Failed to connect wallet';
        const code =
          err && typeof err === 'object' && 'code' in err
            ? String((err as { code?: string }).code)
            : undefined;
        setState((prev) => ({
          ...prev,
          isLoading: false,
          error: { message, code },
        }));
        throw err;
      }
    },
    [expectedNetwork]
  );

  const disconnect = useCallback(async () => {
    const adapterId = state.session?.adapterId;
    setState((prev) => ({ ...prev, isLoading: true, error: null }));
    try {
      await disconnectChainWallet(adapterId);
      setState((prev) => ({
        ...prev,
        session: null,
        networkInfo: null,
        networkMismatch: false,
        isLoading: false,
        error: null,
      }));
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to disconnect wallet';
      setState((prev) => ({
        ...prev,
        isLoading: false,
        error: { message },
      }));
      throw err;
    }
  }, [state.session?.adapterId]);

  const signTransaction = useCallback(
    async (request: SignTransactionRequest) => {
      const adapterId = state.session?.adapterId;
      if (!adapterId) {
        throw new Error('No chain wallet connected');
      }
      if (state.networkMismatch) {
        throw new Error(
          'Wallet network does not match the app. Switch networks before signing.'
        );
      }
      return signWithChainWallet(adapterId, request);
    },
    [state.session?.adapterId, state.networkMismatch]
  );

  const signMessage = useCallback(
    async (request: SignMessageRequest) => {
      const adapterId = state.session?.adapterId;
      if (!adapterId) {
        throw new Error('No chain wallet connected');
      }
      if (state.networkMismatch) {
        throw new Error(
          'Wallet network does not match the app. Switch networks before signing.'
        );
      }
      return signMessageWithChainWallet(adapterId, request);
    },
    [state.session?.adapterId, state.networkMismatch]
  );

  const sendTransaction = useCallback(
    async (request: SendTransactionRequest) => {
      const adapterId = state.session?.adapterId;
      if (!adapterId) {
        throw new Error('No chain wallet connected');
      }
      if (state.networkMismatch) {
        throw new Error(
          'Wallet network does not match the app. Switch networks before sending.'
        );
      }
      return sendWithChainWallet(adapterId, request);
    },
    [state.session?.adapterId, state.networkMismatch]
  );

  const executionSupport = useMemo(() => {
    if (!state.session) return null;
    return resolveExecutionSupport(
      state.session.chainFamily,
      {
        sourceChain: state.session.chainFamily,
        destinationChain: state.session.chainFamily,
      },
      {
        connected: Boolean(state.session.isConnected),
        networkMatch: !state.networkMismatch,
        canSign: Boolean(state.session.isConnected) && !state.networkMismatch,
      }
    );
  }, [state.session, state.networkMismatch]);

  return {
    ...state,
    address: state.session?.account.address ?? null,
    chainFamily: state.session?.chainFamily ?? null,
    adapterId: state.session?.adapterId ?? null,
    isConnected: Boolean(state.session?.isConnected),
    connect,
    disconnect,
    signTransaction,
    signMessage,
    sendTransaction,
    refreshWallets,
    executionSupport,
  };
}
