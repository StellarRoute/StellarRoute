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

export interface PriceHistoryPoint {
  /** Unix timestamp in milliseconds */
  timestamp: number;
  /** Mid-market price as a decimal string */
  price: string;
}

export interface PriceHistoryResponse {
  base_asset: Asset;
  quote_asset: Asset;
  window: "24h";
  source: string;
  /** Unix timestamp in milliseconds */
  generated_at: number;
  points: PriceHistoryPoint[];
}

export type QuoteType = 'sell' | 'buy';

export interface PathStep {
  from_asset: Asset;
  to_asset: Asset;
  price: string;
  /** "sdex" or "amm:<pool_address>" */
  source: string;
  /** Total liquidity depth available at this hop's price */
  liquidity_depth?: string;
  /** Fee in basis points for this hop (e.g., 30 for 0.3%) */
  fee_bps?: number;
}

export interface PriceQuote {
  base_asset: Asset;
  quote_asset: Asset;
  amount: string;
  price: string;
  total: string;
  /** "sell" or "buy" */
  quote_type: QuoteType;
  /** Whether the quote is serving degraded market data */
  degraded?: boolean;
  /** Market midpoint price */
  midpoint?: string;
  /** Market spread in basis points */
  spread_bps?: number;
  /** Route breakdown */
  path: PathStep[];
  priceImpact?: string;
  /** Unix timestamp (seconds) */
  timestamp: number;
  /** Unix timestamp (ms) when this quote expires */
  expires_at?: number;
  /** Unix timestamp (ms) of the underlying data source */
  source_timestamp?: number;
  /** Time-to-live in seconds for client-side staleness detection */
  ttl_seconds?: number;
  /** Estimated price impact percentage */
  price_impact?: string;
  /** Optional alternative routes provided by the aggregator */
  alternativeRoutes?: { id: string; venue: string; expectedAmount: string }[];
}

export interface HealthStatus {
  status: 'healthy' | 'unhealthy';
  version: string;
  /** ISO-8601 UTC timestamp */
  timestamp: string;
  components: Record<string, string>;
}

export type ApiErrorCode =
  | 'internal_error'
  | 'bad_request'
  | 'not_found'
  | 'validation_error'
  | 'rate_limit_exceeded'
  | 'overloaded'
  | 'unauthorized'
  | 'invalid_asset'
  | 'no_route'
  | 'stale_market_data'
  | 'network_error'
  | 'unknown_error';

export interface ApiError {
  error: ApiErrorCode;
  message: string;
  details?: unknown;
}

export interface RouteHop {
  from_asset: Asset;
  to_asset: Asset;
  price: string;
  amount_out_of_hop?: string;
  fee_bps?: number;
  source: string;
}

export interface RouteCandidate {
  score: number;
  impact_bps: number;
  estimated_output: string;
  policy_used?: string;
  path: RouteHop[];
}

export interface RoutesResponse {
  base_asset: Asset;
  quote_asset: Asset;
  amount: string;
  timestamp: number;
  routes: RouteCandidate[];
}

export * from './route';
