/**
 * Stellar asset descriptor returned by the API.
 */
export interface Asset {
  /** Stellar asset type. */
  asset_type: 'native' | 'credit_alphanum4' | 'credit_alphanum12';
  /** Asset code, e.g. `"USDC"`. Absent for native XLM. */
  asset_code?: string;
  /** G-address of the issuing account. Absent for native XLM. */
  asset_issuer?: string;
}

/**
 * A single tradeable asset pair with active orderbook depth.
 */
export interface TradingPair {
  /** Human-readable base asset code, e.g. `"XLM"`. */
  base: string;
  /** Human-readable counter asset code, e.g. `"USDC"`. */
  counter: string;
  /** Canonical base asset identifier (`"native"` or `"CODE:ISSUER"`). */
  base_asset: string;
  /** Canonical counter asset identifier. */
  counter_asset: string;
  /** Number of active offers for this pair. */
  offer_count: number;
  /** RFC-3339 timestamp of the most recent offer update. */
  last_updated?: string;
}

/**
 * Response from `GET /api/v1/pairs`.
 */
export interface PairsResponse {
  /** Active trading pairs ordered by liquidity depth. */
  pairs: TradingPair[];
  /** Total number of pairs returned. */
  total: number;
}

/**
 * A single price level in the orderbook.
 */
export interface OrderbookEntry {
  /** Price as a decimal string (7 decimal places). */
  price: string;
  /** Available amount at this price level. */
  amount: string;
  /** Total value at this price level (`price × amount`). */
  total: string;
}

/**
 * Full orderbook snapshot for a trading pair.
 * Response from `GET /api/v1/orderbook/{base}/{quote}`.
 */
export interface Orderbook {
  base_asset: Asset;
  quote_asset: Asset;
  /** Buy orders sorted highest price first. */
  bids: OrderbookEntry[];
  /** Sell orders sorted lowest price first. */
  asks: OrderbookEntry[];
  /** Unix timestamp of the snapshot. */
  timestamp: number;
}

/**
 * Direction of a price quote.
 * - `"sell"` — how much quote asset you receive when selling `amount` of the base asset.
 * - `"buy"`  — how much base asset you must spend to buy `amount` of the quote asset.
 */
export type QuoteType = 'sell' | 'buy';

/**
 * A single hop in the optimal execution path.
 */
export interface PathStep {
  from_asset: Asset;
  to_asset: Asset;
  /** Exchange rate for this hop. */
  price: string;
  /** Liquidity source: `"sdex"` or `"amm:<pool_address>"`. */
  source: string;
}

/**
 * Best available price quote with full routing path.
 * Response from `GET /api/v1/quote/{base}/{quote}`.
 */
export interface PriceQuote {
  base_asset: Asset;
  quote_asset: Asset;
  /** Input amount that was quoted. */
  amount: string;
  /** Effective price (quote asset per base asset unit). */
  price: string;
  /** Total output amount (`amount × price`). */
  total: string;
  /** Direction of the quote. */
  quote_type: QuoteType;
  /** Ordered list of hops in the optimal execution path. */
  path: PathStep[];
  /** Unix timestamp when the quote was generated. */
  timestamp: number;
  /** Unix timestamp (ms) when this quote expires and should be considered stale */
  expires_at?: number;
  /** Unix timestamp (ms) of the underlying data source (e.g., orderbook snapshot) */
  source_timestamp?: number;
  /** Time-to-live in seconds for client-side staleness detection */
  ttl_seconds?: number;
  /** Estimated price impact percentage */
  price_impact?: string;
  /** Rationale for quote venue selection. */
  rationale?: {
    /** The selection strategy used (e.g., "highest_liquidity", "best_price") */
    strategy: string;
    /** Comparison across different liquidity venues */
    compared_venues: Array<{
      /** Source identifier (e.g., "sdex", "amm:...") */
      source: string;
      /** Quote price from this source */
      price: string;
      /** Total depth available at this price */
      available_amount: string;
      /** Whether the quote was considered executable */
      executable: boolean;
    }>;
  };
}

/**
 * A single request item for a batch quote.
 */
export interface QuoteRequestItem {
  /** Base asset identifier: "native", "CODE", or "CODE:ISSUER". */
  base: string;
  /** Quote asset identifier. */
  quote: string;
  /** Amount to trade (optional). */
  amount?: number;
  /** Direction of the quote ("sell" or "buy"). Defaults to "sell". */
  quote_type?: QuoteType;
}

/**
 * Response from a batch quote request.
 */
export interface BatchQuoteResponse {
  /** Array of quotes in the same order as requested. */
  quotes: PriceQuote[];
  /** Total number of quotes successfully fetched. */
  total: number;
}

/**
 * Configuration for quote staleness detection
 */
export interface QuoteStalenessConfig {
  /** Maximum quote age in seconds before considering stale (default: 30) */
  max_age_seconds: number;
  /** Whether to reject stale quotes on the client side */
  reject_stale?: boolean;
}

/**
 * Default staleness configuration
 */
export const DEFAULT_STALENESS_CONFIG: QuoteStalenessConfig = {
  max_age_seconds: 30,
  reject_stale: false,
};

/**
 * Check if a quote is considered stale
 */
export function isQuoteStale(quote: PriceQuote, config: QuoteStalenessConfig = DEFAULT_STALENESS_CONFIG): boolean {
  const now = Date.now();
  const ageMs = now - quote.timestamp;
  const maxAgeMs = config.max_age_seconds * 1000;
  return ageMs > maxAgeMs;
}

/**
 * Check if a quote has expired based on its expires_at field
 */
export function isQuoteExpired(quote: PriceQuote): boolean {
  if (!quote.expires_at) return false;
  return Date.now() > quote.expires_at;
}

/**
 * Get remaining time until quote expires (in seconds), or null if no expiry
 */
export function getTimeUntilExpiry(quote: PriceQuote): number | null {
  if (!quote.expires_at) return null;
  const remaining = quote.expires_at - Date.now();
  return remaining > 0 ? Math.floor(remaining / 1000) : 0;
}

/**
 * Service health check result.
 * Response from `GET /health`.
 */
export interface HealthStatus {
  /** Overall service status. */
  status: 'healthy' | 'unhealthy';
  /** Deployed package version string. */
  version: string;
  /** ISO-8601 UTC timestamp of the health check. */
  timestamp: string;
  /** Per-dependency health map, e.g. `{ database: "healthy" }`. */
  components: Record<string, string>;
}

/**
 * Optimal trading route without pricing details.
 * Response from `GET /api/v1/route/{base}/{quote}`.
 */
export interface RouteResponse {
  base_asset: Asset;
  quote_asset: Asset;
  /** Input amount being traded. */
  amount: string;
  /** Execution steps for this trade. */
  path: PathStep[];
  /** Slippage tolerance in basis points. */
  slippage_bps: number;
  /** Unix timestamp of the route calculation. */
  timestamp: number;
}

/**
 * Error response from the StellarRoute API.
 */
export interface ApiError {
  /** Machine-readable error code, e.g. `"not_found"`. */
  error: string;
  /** Human-readable description. */
  message: string;
  /** Optional structured context (present on validation errors). */
  details?: unknown;
}

/**
 * Machine-readable error codes returned by the StellarRoute API.
 */
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
  | 'network_error' // SDK specific
  | 'unknown_error' // SDK specific
  | (string & Record<never, never>);
