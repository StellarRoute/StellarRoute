export interface Asset {
  asset_type: 'native' | 'credit_alphanum4' | 'credit_alphanum12';
  asset_code?: string;
  asset_issuer?: string;
}

export interface TradingPair {
  /** Human-readable base asset code, e.g. "XLM" */
  base: string;
  /** Human-readable counter asset code, e.g. "USDC" */
  counter: string;
  /** Canonical base asset identifier: "native" or "CODE:ISSUER" */
  base_asset: string;
  /** Canonical counter asset identifier: "native" or "CODE:ISSUER" */
  counter_asset: string;
  offer_count: number;
  last_updated?: string;
  /** Horizon / ledger numDecimals for base when provided by API */
  base_decimals?: number;
  /** Horizon / ledger numDecimals for counter when provided by API */
  counter_decimals?: number;
}

export interface PairsResponse {
  pairs: TradingPair[];
  total: number;
}

export interface OrderbookEntry {
  price: string;
  amount: string;
  total: string;
}

export interface Orderbook {
  base_asset: Asset;
  quote_asset: Asset;
  bids: OrderbookEntry[];
  asks: OrderbookEntry[];
  /** Unix timestamp (seconds) */
  timestamp: number;
}

export type QuoteType = 'sell' | 'buy';

export interface PathStep {
  from_asset: Asset;
  to_asset: Asset;
  price: string;
  /** "sdex" or "amm:<pool_address>" */
  source: string;
}

export interface PriceQuote {
  base_asset: Asset;
  quote_asset: Asset;
  amount: string;
  price: string;
  total: string;
  quote_type: QuoteType;
  path: PathStep[];
  priceImpact?: string;
  /** Unix timestamp (seconds) */
  timestamp: number;
}

export interface HealthStatus {
  status: 'healthy' | 'unhealthy';
  version: string;
  /** ISO-8601 UTC timestamp */
  timestamp: string;
  components: Record<string, string>;
}

export interface ApiError {
  error: string;
  message: string;
  details?: unknown;
}

export * from './route';
