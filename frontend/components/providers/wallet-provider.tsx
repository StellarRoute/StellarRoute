'use client';

import * as React from 'react';
import {
  connectWallet,
  disconnectWallet,
  getAvailableWallets,
  checkWalletCapabilities,
} from '@/lib/wallet';
import type {
  AvailableWallet,
  SupportedWallet,
  WalletCapabilityStatus,
  WalletError,
  WalletNetwork,
} from '@/lib/wallet/types';

interface WalletContextValue {
  address: string | null;
  isConnected: boolean;
  network: WalletNetwork;
  walletNetwork: WalletNetwork | null;
  walletId: SupportedWallet | null;
  availableWallets: AvailableWallet[];
  isLoading: boolean;
  error: WalletError | null;
  capabilities: WalletCapabilityStatus | null;
  connect: (walletId: SupportedWallet) => Promise<void>;
  disconnect: () => void;
  setNetwork: (network: WalletNetwork) => void;
  refreshWallets: () => Promise<void>;
  refreshCapabilities: () => void;
  networkMismatch: boolean;
  stubSpendableBalance: string | null;
}

const WalletContext = React.createContext<WalletContextValue | undefined>(undefined);

interface WalletProviderProps {
  children: React.ReactNode;
  defaultNetwork?: WalletNetwork;
}

export function WalletProvider({
  children,
  defaultNetwork = 'testnet',
}: WalletProviderProps) {
  const [address, setAddress] = React.useState<string | null>(null);
  const [isConnected, setIsConnected] = React.useState(false);
  const [network, setNetwork] = React.useState<WalletNetwork>(defaultNetwork);
  const [walletNetwork, setWalletNetwork] = React.useState<WalletNetwork | null>(null);
  const [walletId, setWalletId] = React.useState<SupportedWallet | null>(null);
  const [availableWallets, setAvailableWallets] = React.useState<AvailableWallet[]>([]);
  const [capabilities, setCapabilities] = React.useState<WalletCapabilityStatus | null>(null);
  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<WalletError | null>(null);

  const refreshWallets = React.useCallback(async () => {
    const wallets = await getAvailableWallets();
    setAvailableWallets(wallets);
  }, []);

  const refreshCapabilities = React.useCallback(() => {
    if (walletId) {
      const status = checkWalletCapabilities(walletId, network);
      setCapabilities(status);
    } else {
      setCapabilities(null);
    }
  }, [walletId, network]);

  React.useEffect(() => {
    void refreshWallets();
  }, [refreshWallets]);

  React.useEffect(() => {
    refreshCapabilities();
  }, [refreshCapabilities]);

  const connect = React.useCallback(async (selectedWalletId: SupportedWallet) => {
    setIsLoading(true);
    setError(null);
    try {
      const session = await connectWallet(selectedWalletId);
      setAddress(session.address);
      setIsConnected(session.isConnected);
      setWalletNetwork(session.network ?? null);
      setWalletId(session.walletId);
    } catch (err) {
      const e = err instanceof Error ? err : new Error('Unknown error');
      setError({ message: e.message });
    } finally {
      setIsLoading(false);
    }
  }, []);

  const disconnect = React.useCallback(() => {
    const session = disconnectWallet();
    setAddress(session.address);
    setIsConnected(session.isConnected);
    setWalletNetwork(session.network ?? null);
    setWalletId(session.walletId);
    setCapabilities(null);
    setError(null);
  }, []);

  const networkMismatch = isConnected && walletNetwork !== null && walletNetwork !== network;
  const stubSpendableBalance = isConnected ? '10000.0000000' : null;

  const value: WalletContextValue = {
    address,
    isConnected,
    network,
    walletNetwork,
    walletId,
    availableWallets,
    isLoading,
    error,
    capabilities,
    connect,
    disconnect,
    setNetwork,
    refreshWallets,
    refreshCapabilities,
    networkMismatch,
    stubSpendableBalance,
  };

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet() {
  const context = React.useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}