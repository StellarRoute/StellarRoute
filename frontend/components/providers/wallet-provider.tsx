import { createContext, useContext, ReactNode, useState } from 'react';

interface WalletContextType {
'use client';

import * as React from 'react';
import {
  connectWallet,
  disconnectWallet,
  getAvailableWallets,
  refreshWalletSession,
} from '@/lib/wallet';
import type {
  AvailableWallet,
  SupportedWallet,
  WalletError,
  WalletNetwork,
  AccountSwitchState,
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
  connect: (walletId: SupportedWallet) => Promise<void>;
  reconnect: () => Promise<void>;
  disconnect: () => void;
  setNetwork: (network: WalletNetwork) => void;
  autoReconnectPreferred: boolean;
  setAutoReconnectPreferred: (enabled: boolean) => void;
  refreshWallets: () => Promise<void>;
  refreshAccount: () => Promise<void>;
  networkMismatch: boolean;
  stubSpendableBalance: string | null;
  accountSwitchState: AccountSwitchState;
  isTransactionPending: boolean;
  setTransactionPending: (pending: boolean) => void;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

const AUTO_RECONNECT_PREFERENCE_KEY = 'stellarroute.wallet.autoReconnect';
const LAST_WALLET_ID_KEY = 'stellarroute.wallet.lastWalletId';

interface WalletProviderProps {
  children: ReactNode;
  defaultNetwork?: string;
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
  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<WalletError | null>(null);
  const [autoReconnectPreferred, setAutoReconnectPreferredState] = React.useState(true);
  const [didLoadReconnectPreference, setDidLoadReconnectPreference] = React.useState(false);
  const didAttemptInitialReconnect = React.useRef(false);
  const reconnectThrottleUntilMs = React.useRef(0);

  React.useEffect(() => {
    if (typeof window === 'undefined') {
      setDidLoadReconnectPreference(true);
      return;
    }

    const savedPreference = window.localStorage.getItem(
      AUTO_RECONNECT_PREFERENCE_KEY,
    );
    if (savedPreference !== null) {
      setAutoReconnectPreferredState(savedPreference === 'true');
    }
    setDidLoadReconnectPreference(true);
  }, []);

  const setAutoReconnectPreferred = React.useCallback((enabled: boolean) => {
    setAutoReconnectPreferredState(enabled);
    if (typeof window === 'undefined') {
      return;
    }
    window.localStorage.setItem(AUTO_RECONNECT_PREFERENCE_KEY, String(enabled));
  }, []);

  const setLastWalletId = React.useCallback((id: SupportedWallet | null) => {
    if (typeof window === 'undefined') {
      return;
    }
    if (id === null) {
      window.localStorage.removeItem(LAST_WALLET_ID_KEY);
      return;
    }
    window.localStorage.setItem(LAST_WALLET_ID_KEY, id);
  }, []);

  const refreshWallets = React.useCallback(async () => {
    const wallets = await getAvailableWallets();
    setAvailableWallets(wallets);
  }, []);

  React.useEffect(() => {
    void refreshWallets();
  }, [refreshWallets]);

  const connect = React.useCallback(async (selectedWalletId: SupportedWallet) => {
    // Prevent account switching during transactions
    if (isTransactionPending) {
      setError({ message: 'Cannot switch accounts during a pending transaction' });
      return;
    }

    setIsLoading(true);
    setError(null);
    setAccountSwitchState({
      isDetecting: false,
      hasChanged: false,
      previousAddress: null,
    });

    try {
      const session = await connectWallet(selectedWalletId);
      setAddress(session.address);
      setIsConnected(session.isConnected);
      setWalletNetwork(session.network ?? null);
      setWalletId(session.walletId);
      setLastWalletId(session.walletId);
    } catch (err) {
      const e = err instanceof Error ? err : new Error('Unknown error');
      setError({ message: e.message });
    } finally {
      setIsLoading(false);
    }
  }, [setLastWalletId]);

  const reconnect = React.useCallback(async () => {
    if (typeof window === 'undefined') {
      return;
    }
    const savedWalletId = window.localStorage.getItem(LAST_WALLET_ID_KEY);
    if (savedWalletId !== 'freighter' && savedWalletId !== 'xbull') {
      return;
    }

    const available =
      availableWallets.find((wallet) => wallet.id === savedWalletId) ?? null;
    if (available && !available.installed) {
      setError({ message: `${available.label} is not installed.` });
      return;
    }

    await connect(savedWalletId);
  }, [availableWallets, connect]);

  const disconnect = React.useCallback(() => {
    // Prevent disconnection during transactions
    if (isTransactionPending) {
      setError({ message: 'Cannot disconnect during a pending transaction' });
      return;
    }

    const session = disconnectWallet();
    setAddress(session.address);
    setIsConnected(session.isConnected);
    setWalletNetwork(session.network ?? null);
    setWalletId(session.walletId);
    setError(null);
    setAccountSwitchState({
      isDetecting: false,
      hasChanged: false,
      previousAddress: null,
    });
  }, [isTransactionPending]);

  const refreshAccount = React.useCallback(async () => {
    if (!walletId || !isConnected) return;

    // Prevent account refresh during transactions
    if (isTransactionPending) {
      setError({ message: 'Cannot refresh account during a pending transaction' });
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const session = await refreshWalletSession(walletId);
      const previousAddress = address;
      
      setAddress(session.address);
      setIsConnected(session.isConnected);
      setWalletNetwork(session.network ?? null);
      setWalletId(session.walletId);

      // Reset account switch state after successful refresh
      setAccountSwitchState({
        isDetecting: false,
        hasChanged: false,
        previousAddress: null,
      });

      // If address changed, this was an account switch
      if (previousAddress && session.address !== previousAddress) {
        // Trigger any necessary balance/quote refreshes here
        console.log('Account switched from', previousAddress, 'to', session.address);
      }
    } catch (err) {
      const e = err instanceof Error ? err : new Error('Unknown error');
      setError({ message: e.message });
    } finally {
      setIsLoading(false);
    }
  }, [walletId, isConnected, address, isTransactionPending]);

  React.useEffect(() => {
    if (!didLoadReconnectPreference) {
      return;
    }
    if (didAttemptInitialReconnect.current) {
      return;
    }
    didAttemptInitialReconnect.current = true;
    if (!autoReconnectPreferred || isConnected) {
      return;
    }
    void reconnect();
  }, [autoReconnectPreferred, didLoadReconnectPreference, isConnected, reconnect]);

  React.useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const tryRecoverConnection = () => {
      if (!autoReconnectPreferred || isConnected || isLoading) {
        return;
      }
      const now = Date.now();
      if (now < reconnectThrottleUntilMs.current) {
        return;
      }
      reconnectThrottleUntilMs.current = now + 5000;
      void reconnect();
    };

    window.addEventListener('focus', tryRecoverConnection);
    window.addEventListener('online', tryRecoverConnection);

    return () => {
      window.removeEventListener('focus', tryRecoverConnection);
      window.removeEventListener('online', tryRecoverConnection);
    };
  }, [autoReconnectPreferred, isConnected, isLoading, reconnect]);

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
    connect,
    reconnect,
    disconnect,
    setNetwork,
    autoReconnectPreferred,
    setAutoReconnectPreferred,
    refreshWallets,
    refreshAccount,
    networkMismatch,
    stubSpendableBalance,
    accountSwitchState,
    isTransactionPending,
    setTransactionPending: React.useCallback((pending: boolean) => {
      setIsTransactionPending(pending);
    }, []),
  };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within WalletProvider');
  }
  return context;
}
