"use client";

import { useEffect, useMemo, useState } from "react";
import {
  connectWallet,
  disconnectWallet,
  getAvailableWallets,
  signTransactionStub,
} from "@/lib/wallet";
import type { SupportedWallet, WalletSession } from "@/lib/wallet/types";

const initialState: WalletSession = {
  walletId: null,
  address: null,
  network: null,
  isConnected: false,
};

export function useWallet() {
  const [session, setSession] = useState<WalletSession>(initialState);
  const [availableWallets, setAvailableWallets] = useState<
    { id: SupportedWallet; label: string }[]
  >([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 🔹 Load available wallets on mount
  useEffect(() => {
    loadWallets();
  }, []);

  const loadWallets = async () => {
    try {
      const wallets = await getAvailableWallets();
      setAvailableWallets(wallets);
      return wallets;
    } catch {
      setAvailableWallets([]);
    }
  };

  // 🔹 Connect wallet
  const connect = async (walletId: SupportedWallet) => {
    try {
      setLoading(true);
      setError(null);

      const next = await connectWallet(walletId);

      setSession({
        walletId: next.walletId,
        address: next.address,
        network: next.network,
        isConnected: true,
      });
   } catch (e: unknown) {
  const msg = e instanceof Error ? e.message.toLowerCase() : "";

      if (msg.includes("reject")) {
        setError("Connection request was rejected.");
      } else if (msg.includes("lock")) {
        setError("Wallet is locked. Unlock it and try again.");
      } else if (msg.includes("not installed")) {
        setError("Wallet not installed.");
      } else {
        setError("Unable to connect wallet.");
      }
    } finally {
      setLoading(false);
    }
  };

  // 🔹 Disconnect wallet
  const disconnect = () => {
    disconnectWallet();
    setSession(initialState);
    setError(null);
  };

  // 🔹 Short address (GABC...WXYZ)
  const shortAddress = useMemo(() => {
    if (!session.address) return "";
    return `${session.address.slice(0, 4)}...${session.address.slice(-4)}`;
  }, [session.address]);

  // 🔹 Copy full address
  const copyAddress = async () => {
    if (!session.address) return;

    try {
      await navigator.clipboard.writeText(session.address);
    } catch {
      setError("Failed to copy address.");
    }
  };

  return {
    session,
    availableWallets,
    loading,
    error,
    shortAddress,
    connect,
    disconnect,
    copyAddress,
    signTransactionStub, // required by issue
  };
}