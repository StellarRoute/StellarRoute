"use client";

import { useCallback, useEffect, useState } from "react";
import { useWallet } from "@/components/providers/wallet-provider";

interface AssetBalance {
  assetType: "native" | "credit_alphanum4" | "credit_alphanum12";
  assetCode?: string;
  assetIssuer?: string;
  balance: string;
}

interface WalletBalances {
  [key: string]: string; // key is asset identifier: "native" or "CODE:ISSUER"
}

export function useWalletBalance() {
  const { address, isConnected, network } = useWallet();
  const [balances, setBalances] = useState<WalletBalances>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getHorizonUrl = useCallback(() => {
    if (network === "mainnet") {
      return "https://horizon.stellar.org";
    }
    return "https://horizon-testnet.stellar.org";
  }, [network]);

  const fetchBalances = useCallback(async () => {
    if (!isConnected || !address) {
      setBalances({});
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const horizonUrl = getHorizonUrl();
      const response = await fetch(`${horizonUrl}/accounts/${address}`);
      
      if (!response.ok) {
        throw new Error(`Failed to fetch account: ${response.statusText}`);
      }

      const accountData = await response.json();
      const newBalances: WalletBalances = {};

      accountData.balances.forEach((balance: AssetBalance) => {
        if (balance.assetType === "native") {
          newBalances["native"] = balance.balance;
        } else {
          const key = `${balance.assetCode}:${balance.assetIssuer}`;
          newBalances[key] = balance.balance;
        }
      });

      setBalances(newBalances);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to fetch balances";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [isConnected, address, getHorizonUrl]);

  useEffect(() => {
    fetchBalances();
  }, [fetchBalances]);

  const getBalance = useCallback(
    (asset: string) => {
      return balances[asset] || null;
    },
    [balances]
  );

  return {
    balances,
    loading,
    error,
    getBalance,
    refresh: fetchBalances,
  };
}
