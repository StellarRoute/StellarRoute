export type OfframpMode = 'direct' | 'bridge';

export type OfframpSourceKind =
  | 'stellar_usdc'
  | 'stellar_xlm'
  | 'evm_usdc'
  | 'solana_usdc'
  | 'other_stable';

export type OfframpSourceStatus = 'ready' | 'bridge_required' | 'swap_then_offramp' | 'coming_soon';

export interface OfframpSourceAsset {
  id: string;
  symbol: string;
  name: string;
  chainLabel: string;
  kind: OfframpSourceKind;
  status: OfframpSourceStatus;
  /** True when the asset already sits on Stellar as native USDC. */
  isStellarUsdc: boolean;
  decimals: number;
  /** Short hint under the picker row. */
  hint: string;
}

export type FiatCurrencyCode = 'NGN';

export interface FiatCurrency {
  code: FiatCurrencyCode;
  name: string;
  symbol: string;
  country: string;
  /** Flag emoji for lightweight visual cue. */
  flag: string;
}

export interface NigerianBank {
  code: string;
  name: string;
}

export interface OfframpDestination {
  fiat: FiatCurrencyCode;
  bankCode: string;
  accountNumber: string;
  accountName: string;
}

export type OfframpStepId =
  | 'source'
  | 'bridge'
  | 'settle_usdc'
  | 'payout';

export interface OfframpRouteStep {
  id: OfframpStepId;
  label: string;
  detail: string;
  active: boolean;
  optional?: boolean;
}

export interface OfframpQuotePreview {
  sourceAmount: string;
  sourceSymbol: string;
  usdcAmount: string;
  feeUsdc: string;
  netUsdc: string;
  rateNgn: number;
  receiveNgn: string;
  etaLabel: string;
  mode: OfframpMode;
  indicative: true;
}
