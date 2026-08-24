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
  parseApiErrorBody,
  DEFAULT_TESTNET_HORIZON_URL,
} from './client.js';

export type { StellarRouteClientOptions } from './client.js';

export type {
  ApiError,
  ApiErrorCode,
  ApiV2Info,
  Asset,
  BridgeVenueMeta,
  BatchItemError,
  CanonicalizeAssetResponse,
  ChainAsset,
  ChainNamespace,
  BatchOrderbookItemResult,
  BatchOrderbookResponse,
  BatchQuoteResponse,
  ExcludedVenueInfo,
  ExclusionDiagnostics,
  ExclusionReason,
  ExecuteSwapNetworkPassphrase,
  ExecuteSwapParams,
  ExecuteSwapResult,
  HealthStatus,
  Orderbook,
  OrderbookEntry,
  OrderbookRequestItem,
  PairsResponse,
  PathStep,
  PreparedSwapResponse,
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
  SwapConfirmResult,
  SwapExecutionMode,
  SwapPrepareRequest,
  SwapSubmitRequest,
  SwapSubmitResponse,
  TradingPair,
  SupportedCorridor,
  CctpDirection,
  CctpFinality,
  CctpTransferStatus,
  CctpFeeQuote,
  PreparedWalletPayload,
  CctpStatusDetails,
  CctpQuoteRequest,
  CctpQuoteResponse,
  CctpTransferStatusResponse,
  CctpPrepareBurnResponse,
  CctpSubmitBurnRequest,
  CctpSubmitBurnResponse,
  CctpPrepareMintResponse,
  CctpSubmitMintRequest,
  CctpSubmitMintResponse,
  CctpReattestResponse,
} from './types.js';

export {
  API_ERROR_CODES,
  DEFAULT_STALENESS_CONFIG,
  assetToCanonical,
  canonicalizeAssetId,
  looksLikeCaip,
  isQuoteStale,
  isQuoteExpired,
  getTimeUntilExpiry,
  pathStepToSimulationHop,
  stellarAssetToCanonical,
  CCTP_PROVIDER_ID,
  CCTP_TESTNET_CORRIDOR_ID,
} from './types.js';

export * from './websocket.js';
