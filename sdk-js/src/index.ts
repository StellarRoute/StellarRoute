/**
 * @stellarroute/sdk-js
 *
 * TypeScript SDK for the StellarRoute DEX aggregation API.
 *
 * @example
 * ```ts
 * import { StellarRouteClient, isStellarRouteApiError } from '@stellarroute/sdk-js';
 *
 * const client = new StellarRouteClient({ baseUrl: 'https://api.stellarroute.io' });
 *
 * try {
 *   const quote = await client.getQuote('native', 'USDC', 100);
 *   console.log(quote.price);
 * } catch (err) {
 *   if (isStellarRouteApiError(err) && err.isNotFound()) {
 *     console.log('no route found for this pair');
 *   }
 * }
 * ```
 *
 * @packageDocumentation
 */

export {
  StellarRouteClient,
  StellarRouteApiError,
  isStellarRouteApiError,
} from './client.js';

export type { StellarRouteClientOptions } from './client.js';

export type {
  ApiError,
  ApiErrorCode,
  Asset,
  BatchItemError,
  BatchOrderbookItemResult,
  BatchOrderbookResponse,
  BatchQuoteResponse,
  ExcludedVenueInfo,
  ExclusionDiagnostics,
  ExclusionReason,
  ExecuteSwapParams,
  ExecuteSwapResult,
  HealthStatus,
  Orderbook,
  OrderbookEntry,
  OrderbookRequestItem,
  PairsResponse,
  PathStep,
  PriceHistoryPoint,
  PriceHistoryResponse,
  PriceHistoryWindow,
  PriceQuote,
  QuoteRequestItem,
  QuoteStalenessConfig,
  QuoteType,
  RankedRouteCandidate,
  RankedRouteHop,
  RankedRoutesResponse,
  SimulateRouteRequest,
  SimulateRouteResponse,
  SimulationHop,
  SimulationSlippageOverride,
  TradingPair,
} from './types.js';

export {
  DEFAULT_STALENESS_CONFIG,
  isQuoteStale,
  isQuoteExpired,
  getTimeUntilExpiry,
} from './types.js';

export * from './websocket.js';
