'use client';

import { useEffect, useMemo, useState } from 'react';
import type { WalletNetwork } from '@/lib/wallet/types';
import { XLM_FEE_RESERVE } from '@/lib/stellar-reserves';

interface HorizonBalanceLine {
  balance: string;
  asset_type: string;
  asset_code?: string;
  asset_issuer?: string;
}

interface HorizonAccountResponse {
  balances?: HorizonBalanceLine[];
}

interface WalletBalanceState {
  balance: string | null;
  spendableBalance: string | null;
  loading: boolean;
  error: Error | null;
}

const HORIZON_URLS: Record<string, string> = {
  testnet: 'https://horizon-testnet.stellar.org',
  mainnet: 'https://horizon.stellar.org',
};

function normalizeNetwork(network: WalletNetwork | null): string {
  const defaultNetwork = process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';
  return String(network ?? defaultNetwork).toLowerCase();
}

function findAssetBalance(
  balances: HorizonBalanceLine[],
  asset: string,
): string {
  if (asset === 'native') {
    return balances.find((line) => line.asset_type === 'native')?.balance ?? '0';
  }

  const [code, issuer] = asset.split(':');
  if (!code || !issuer) return '0';

  return (
    balances.find(
      (line) => line.asset_code === code && line.asset_issuer === issuer,
    )?.balance ?? '0'
  );
}

function formatSpendableNativeBalance(balance: string): string {
  const value = Number.parseFloat(balance);
  if (!Number.isFinite(value)) return '0';
  return Math.max(0, value - XLM_FEE_RESERVE).toFixed(7);
}

export function useWalletBalance({
  address,
  asset,
  isConnected,
  network,
}: {
  address: string | null;
  asset: string;
  isConnected: boolean;
  network: WalletNetwork | null;
}): WalletBalanceState {
  const [state, setState] = useState<WalletBalanceState>({
    balance: null,
    spendableBalance: null,
    loading: false,
    error: null,
  });

  const networkKey = normalizeNetwork(network);

  useEffect(() => {
    if (!isConnected || !address) {
      setState({
        balance: null,
        spendableBalance: null,
        loading: false,
        error: null,
      });
      return;
    }

    const defaultNetwork = process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';
    const horizonUrl = networkKey === defaultNetwork.toLowerCase() && process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL
      ? process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL
      : HORIZON_URLS[networkKey];
    if (!horizonUrl) {
      setState({
        balance: null,
        spendableBalance: null,
        loading: false,
        error: new Error(`Unsupported network: ${network}`),
      });
      return;
    }

    const controller = new AbortController();
    setState((previous) => ({ ...previous, loading: true, error: null }));

    fetch(`${horizonUrl}/accounts/${encodeURIComponent(address)}`, {
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error('Unable to load wallet balance.');
        }
        return (await response.json()) as HorizonAccountResponse;
      })
      .then((account) => {
        if (controller.signal.aborted) return;
        const balance = findAssetBalance(account.balances ?? [], asset);
        setState({
          balance,
          spendableBalance:
            asset === 'native' ? formatSpendableNativeBalance(balance) : balance,
          loading: false,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        setState({
          balance: null,
          spendableBalance: null,
          loading: false,
          error: error instanceof Error ? error : new Error(String(error)),
        });
      });

    return () => controller.abort();
  }, [address, asset, isConnected, network, networkKey]);

  return useMemo(() => state, [state]);
}
