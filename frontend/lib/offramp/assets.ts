import type { FiatCurrency, OfframpSourceAsset } from './types';

export const OFFRAMP_FIAT: FiatCurrency = {
  code: 'NGN',
  name: 'Nigerian Naira',
  symbol: '₦',
  country: 'Nigeria',
  flag: '🇳🇬',
};

/** Selectable source assets for the first offramp corridor. */
export const OFFRAMP_SOURCE_ASSETS: OfframpSourceAsset[] = [
  {
    id: 'stellar-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Stellar',
    kind: 'stellar_usdc',
    status: 'ready',
    isStellarUsdc: true,
    decimals: 7,
    hint: 'Offramp directly — no bridge step.',
  },
  {
    id: 'stellar-xlm',
    symbol: 'XLM',
    name: 'Lumens',
    chainLabel: 'Stellar',
    kind: 'stellar_xlm',
    status: 'swap_then_offramp',
    isStellarUsdc: false,
    decimals: 7,
    hint: 'Swap to Stellar USDC, then cash out to Naira.',
  },
  {
    id: 'eth-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Ethereum',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'base-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Base',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'arb-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Arbitrum',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'sol-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Solana',
    kind: 'solana_usdc',
    status: 'coming_soon',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Solana CCTP corridor coming soon.',
  },
];

export const DEFAULT_OFFRAMP_SOURCE_ID = 'stellar-usdc';

export function findOfframpSource(
  id: string,
): OfframpSourceAsset | undefined {
  return OFFRAMP_SOURCE_ASSETS.find((asset) => asset.id === id);
}

export function resolveOfframpMode(
  asset: OfframpSourceAsset,
): 'direct' | 'bridge' {
  return asset.isStellarUsdc ? 'direct' : 'bridge';
}
