import type { AdapterNetworkId } from '../types';
import { EVM_NETWORKS, getAddChainParams } from './networks';

/** Reown / WalletConnect Cloud project id (public). */
export function getWalletConnectProjectId(): string | null {
  const value = process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID?.trim();
  return value ? value : null;
}

export function isWalletConnectConfigured(): boolean {
  return Boolean(getWalletConnectProjectId());
}

export function getWalletConnectMetadata(): {
  name: string;
  description: string;
  url: string;
  icons: string[];
} {
  const origin =
    typeof window !== 'undefined' && window.location?.origin
      ? window.location.origin
      : 'https://stellarroute.app';
  return {
    name: 'StellarRoute',
    description:
      'Non-custodial Stellar DEX aggregator with cross-chain CCTP bridges',
    url: origin,
    icons: [`${origin}/icons/icon-192.svg`],
  };
}

/** Decimal chain ids for WalletConnect optionalChains (Sepolia first). */
export function getWalletConnectOptionalChains(): [number, ...number[]] {
  return [
    11155111, // Ethereum Sepolia (CCTP test corridor)
    1,
    8453,
    84532,
    42161,
    421614,
  ];
}

export function caip2ToChainIdDecimal(
  network: AdapterNetworkId
): number | null {
  if (!network.startsWith('eip155:')) return null;
  const id = network.slice('eip155:'.length);
  const decimal = Number.parseInt(id, 10);
  return Number.isFinite(decimal) ? decimal : null;
}

export function getWalletConnectRpcMap(): Record<string, string> {
  const map: Record<string, string> = {};
  for (const network of Object.values(EVM_NETWORKS)) {
    const params = getAddChainParams(network);
    const decimal = caip2ToChainIdDecimal(network);
    if (!params || decimal === null) continue;
    const rpc = params.rpcUrls[0];
    if (rpc) map[String(decimal)] = rpc;
  }
  return map;
}
