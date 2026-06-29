import { useCallback, useMemo } from 'react';
import { useWallet as useWalletContext } from '@/components/providers/wallet-provider';

/**
 * Legacy useWallet hook wrapper for backward compatibility.
 */
export function useWallet() {
  const context = useWalletContext();

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

  return {
    session,
    availableWallets: context.availableWallets,
    loading: context.isLoading,
    error: context.error ? context.error.message : null,
    shortAddress,
    connect: context.connect,
    disconnect: context.disconnect,
    copyAddress,
  };
}
