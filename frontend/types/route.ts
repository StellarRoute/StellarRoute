import { PathStep } from './index';

/**
 * Split-path shape returned by the quote API. The parser accepts both the
 * current snake_case payload and the older SDK aliases during rollout.
 */
export interface ApiSplitPath {
  path?: PathStep[];
  steps?: PathStep[];
  percentage?: number;
  allocation_percent?: number;
  allocation_bps?: number;
  weight?: number;
  output_amount?: string;
  outputAmount?: string;
}

export interface SplitRouteQuotePayload {
  split_paths?: ApiSplitPath[];
  splitPaths?: ApiSplitPath[];
  routes?: ApiSplitPath[];
}

export interface SplitPath {
  /** Percentage of trade allocated to this path (0-100) */
  percentage: number;
  /** The route steps for this path */
  steps: PathStep[];
  /** Expected output amount for this path */
  outputAmount?: string;
}

export interface SplitRouteData {
  /** Array of parallel paths */
  paths: SplitPath[];
  /** Total expected output across all paths */
  totalOutput: string;
  /** Total fees across all paths */
  totalFees?: string;
  /** Total price impact across all paths */
  totalPriceImpact?: string;
}

export interface RouteMetrics {
  /** Total fees paid across the route */
  totalFees: string;
  /** Total price impact percentage */
  totalPriceImpact: string;
  /** Net output amount after fees */
  netOutput: string;
  /** Average exchange rate */
  averageRate: string;
}
