import { useCallback, useMemo } from 'react';
import { useWallet as useWalletContext } from '@/components/providers/wallet-provider';
import type { SupportedWallet } from '@/lib/wallet/types';

/**
 * Legacy useWallet hook wrapper for backward compatibility.
 */
export function useWallet() {
  const context = useWalletContext();
  const { connect: connectContext } = context;

  const session = useMemo(() => ({
    isConnected: context.isConnected,
    address: context.address,
    network: context.walletNetwork,
    walletId: context.walletId,
  }), [context.isConnected, context.address, context.walletNetwork, context.walletId]);

  const shortAddress = useMemo(() => {
    if (!context.address) return '';
    return `${context.address.slice(0, 4)}...${context.address.slice(-4)}`;
  }, [context.address]);

  const copyAddress = useCallback(async () => {
    if (!context.address) return;
    try {
      await navigator.clipboard.writeText(context.address);
    } catch (err) {
      console.error('Failed to copy address:', err);
    }
  }, [context.address]);

  const connect = useCallback(
    async (walletId: SupportedWallet) => {
      try {
        await connectContext(walletId);
      } catch {
        // The legacy hook exposed connection failures through `error` state
        // rather than rejecting from connect(). Preserve that contract for
        // older callers while WalletProvider remains free to throw.
      }
    },
    [connectContext],
  );

  return {
    session,
    availableWallets: context.availableWallets,
    loading: context.isLoading,
    error: context.error ? context.error.message : null,
    shortAddress,
    connect,
    disconnect: context.disconnect,
    copyAddress,
  };
}
