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
  ExcludedVenueInfo,
  ExclusionDiagnostics,
  ExclusionReason,
  HealthStatus,
  Orderbook,
  OrderbookEntry,
  PairsResponse,
  PathStep,
  PriceHistoryPoint,
  PriceHistoryResponse,
  PriceHistoryWindow,
  PriceQuote,
  QuoteStalenessConfig,
  QuoteType,
  RankedRouteCandidate,
  RankedRouteHop,
  RankedRoutesResponse,
  TradingPair,
} from './types.js';

export {
  DEFAULT_STALENESS_CONFIG,
  isQuoteStale,
  isQuoteExpired,
  getTimeUntilExpiry,
} from './types.js';

export * from './websocket.js';
