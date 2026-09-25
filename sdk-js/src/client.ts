import type {
  ApiErrorCode,
  ExecuteSwapParams,
  ExecuteSwapResult,
  HealthStatus,
  Orderbook,
  PairsResponse,
  PathStep,
  PriceHistoryResponse,
  PriceHistoryWindow,
  PriceQuote,
  QuoteRequestItem,
  BatchQuoteResponse,
  OrderbookRequestItem,
  BatchOrderbookResponse,
  QuoteStalenessConfig,
  QuoteType,
  RankedRoutesResponse,
  RouteResponse,
  SimulateRouteRequest,
  SimulateRouteResponse,
  PreparedSwapResponse,
  SwapConfirmResult,
  SwapPrepareRequest,
  SwapSubmitRequest,
  SwapSubmitResponse,
  ApiV2Info,
  SupportedCorridor,
  CctpQuoteRequest,
  CctpQuoteResponse,
  CctpCallOptions,
  CctpTransferStatusResponse,
  CctpPrepareBurnResponse,
  CctpSubmitBurnRequest,
  CctpSubmitBurnResponse,
  CctpPrepareMintResponse,
  CctpSubmitMintRequest,
  CctpSubmitMintResponse,
  CctpReattestResponse,
} from './types.js';
import {
  DEFAULT_STALENESS_CONFIG,
  isQuoteStale,
  isQuoteExpired,
  CCTP_TRANSFER_ACCESS_HEADER,
  CCTP_IDEMPOTENCY_HEADER,
} from './types.js';

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL = 'http://localhost:8080';
const DEFAULT_TIMEOUT_MS = 10_000;
const DEFAULT_RETRIES = 2;
/** Default Horizon testnet URL used by {@link StellarRouteClient.confirmSwap}. */
export const DEFAULT_TESTNET_HORIZON_URL = 'https://horizon-testnet.stellar.org';
const CLASSIC_EXECUTION_MODE = 'classic_path_payment';

/**
 * Unwrap `{ data: T }` API envelopes when present; otherwise return the body as-is.
 */
function unwrapApiData<T>(body: unknown): T {
  if (
    body !== null &&
    typeof body === 'object' &&
    'data' in body &&
    (body as { data: unknown }).data !== undefined
  ) {
    return (body as { data: T }).data;
  }
  return body as T;
}

/**
 * Parse API error payloads in either flat `{ error, message, details }` form or
 * the Axum envelope `{ data: { error, message, details } }`.
 */
export function parseApiErrorBody(body: unknown): {
  error?: string;
  message?: string;
  details?: unknown;
} {
  if (!body || typeof body !== 'object') return {};

  const root = body as {
    error?: unknown;
    message?: unknown;
    details?: unknown;
    data?: unknown;
  };

  const nested =
    root.data && typeof root.data === 'object'
      ? (root.data as {
          error?: unknown;
          message?: unknown;
          details?: unknown;
        })
      : null;

  const source =
    typeof root.error === 'string'
      ? root
      : nested && typeof nested.error === 'string'
        ? nested
        : root.error || nested?.error
          ? nested ?? root
          : nested ?? root;

  return {
    error: typeof source.error === 'string' ? source.error : undefined,
    message: typeof source.message === 'string' ? source.message : undefined,
    details: source.details,
  };
}

function isAmbiguousSubmitError(err: StellarRouteApiError): boolean {
  return (
    err.code === 'dependency_unavailable' ||
    err.code === 'network_error' ||
    err.status === 503 ||
    err.status === 0
  );
}

// ── Error class ───────────────────────────────────────────────────────────────

/**
 * Thrown by {@link StellarRouteClient} for any non-2xx API response or
 * network failure.
 *
 * @example
 * ```ts
 * try {
 *   await client.getOrderbook('native', 'GHOST');
 * } catch (err) {
 *   if (isStellarRouteApiError(err) && err.isNotFound()) {
 *     console.log('pair not found');
 *   }
 * }
 * ```
 */
export class StellarRouteApiError extends Error {
  /** HTTP status code. `0` for network-level failures. */
  public readonly status: number;
  /** Machine-readable error code from the API response body. */
  public readonly code: ApiErrorCode;
  /** Optional structured context from the API response body. */
  public readonly details?: unknown;

  constructor(
    status: number,
    code: ApiErrorCode,
    message: string,
    details?: unknown,
  ) {
    super(message);
    this.name = 'StellarRouteApiError';
    this.status = status;
    this.code = code;
    this.details = details;
  }

  /** Returns `true` when the API returned 404 Not Found. */
  isNotFound(): boolean {
    return this.status === 404 || this.code === 'not_found';
  }

  /** Returns `true` when the request was rate-limited (HTTP 429). */
  isRateLimited(): boolean {
    return this.status === 429 || this.code === 'rate_limit_exceeded';
  }

  /** Returns `true` when the service is overloaded (HTTP 503). */
  isOverloaded(): boolean {
    return this.status === 503 || this.code === 'overloaded';
  }

  /** Returns `true` when the market data is stale (HTTP 422). */
  isStaleMarketData(): boolean {
    return this.status === 422 || this.code === 'stale_market_data';
  }

  /** Returns `true` for bad-request validation errors (HTTP 400). */
  isValidationError(): boolean {
    return (
      this.status === 400 ||
      this.code === 'validation_error' ||
      this.code === 'invalid_asset'
    );
  }

  /** Returns `true` for network-level failures (no HTTP response). */
  isNetworkError(): boolean {
    return this.status === 0;
  }
}

/**
 * Type guard — returns `true` when `err` is a {@link StellarRouteApiError}.
 */
export function isStellarRouteApiError(err: unknown): err is StellarRouteApiError {
  return err instanceof StellarRouteApiError;
}

// ── Client options ────────────────────────────────────────────────────────────

/**
 * Options accepted by the {@link StellarRouteClient} constructor.
 */
export interface StellarRouteClientOptions {
  /**
   * Base URL of the StellarRoute API.
   * @default "http://localhost:8080"
   */
  baseUrl?: string;
  /**
   * Request timeout in milliseconds.
   * @default 10_000
   */
  timeoutMs?: number;
  /**
   * Number of automatic retries on 429 / 5xx / network errors.
   * @default 2
   */
  retries?: number;
  /**
   * Additional headers sent with every request.
   */
  headers?: Record<string, string>;
}

// ── Client ────────────────────────────────────────────────────────────────────

/**
 * Async HTTP client for the StellarRoute REST API.
 *
 * @example
 * ```ts
 * import { StellarRouteClient } from '@stellarroute/sdk-js';
 *
 * const client = new StellarRouteClient({ baseUrl: 'https://api.stellarroute.io' });
 *
 * const health = await client.getHealth();
 * console.log(health.status); // "healthy"
 *
 * const quote = await client.getQuote('native', 'USDC', 100);
 * console.log(quote.price);
 * ```
 */
export class StellarRouteClient {
  private readonly baseUrl: string;
  private readonly timeoutMs: number;
  private readonly retries: number;
  private readonly extraHeaders: Record<string, string>;

  constructor(options: StellarRouteClientOptions | string = {}) {
    // Accept a plain string for backward compatibility.
    if (typeof options === 'string') {
      options = { baseUrl: options };
    }
    this.baseUrl = (options.baseUrl ?? DEFAULT_BASE_URL).replace(/\/$/, '');
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.retries = options.retries ?? DEFAULT_RETRIES;
    this.extraHeaders = options.headers ?? {};
  }

  // ── Public API methods ──────────────────────────────────────────────────────

  /**
   * `GET /health` — probe service and dependency health.
   */
  getHealth(signal?: AbortSignal): Promise<HealthStatus> {
    return this.request<HealthStatus>('/health', signal);
  }

  /**
   * `GET /api/v1/pairs` — list active trading pairs.
   */
  getPairs(signal?: AbortSignal): Promise<PairsResponse> {
    return this.request<PairsResponse>('/api/v1/pairs', signal);
  }

  /**
   * `GET /api/v1/orderbook/{base}/{quote}` — fetch orderbook snapshot.
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when the pair
   *   has no active offers.
   */
  getOrderbook(
    base: string,
    quote: string,
    signal?: AbortSignal,
  ): Promise<Orderbook> {
    const path = `/api/v1/orderbook/${encodeURIComponent(base)}/${encodeURIComponent(quote)}`;
    return this.request<Orderbook>(path, signal);
  }

  /**
   * `POST /api/v1/batch/orderbook` — fetch multiple orderbooks in a single request.
   *
   * @throws {@link StellarRouteApiError} when the batch request fails.
   */
  async getOrderbooksBatch(
    requests: OrderbookRequestItem[],
    signal?: AbortSignal,
  ): Promise<BatchOrderbookResponse> {
    const path = '/api/v1/batch/orderbook';
    return this.request<BatchOrderbookResponse>(
      path,
      signal,
      this.retries,
      'POST',
      { requests },
    );
  }

  /**
   * `GET /api/v1/quote/{base}/{quote}` — get best price quote.
   *
   * @param base   Base asset identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`.
   * @param quote  Quote asset identifier.
   * @param amount Amount of the base asset to trade. Defaults to `1`.
   * @param type   Direction of the quote (`"sell"` or `"buy"`). Defaults to `"sell"`.
   * @param slippage Slippage tolerance in basis points (e.g. 50 = 0.5%).
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when no route exists.
   * @throws {@link StellarRouteApiError} with `status === 400` for invalid params.
   */
  getQuote(
    base: string,
    quote: string,
    amount?: number,
    type: QuoteType = 'sell',
    slippage?: number,
    signal?: AbortSignal,
  ): Promise<PriceQuote> {
    const params = new URLSearchParams({ quote_type: type });
    if (amount !== undefined) params.set('amount', String(amount));
    if (slippage !== undefined) params.set('slippage_bps', String(slippage));
    const path = `/api/v1/quote/${encodeURIComponent(base)}/${encodeURIComponent(quote)}?${params}`;
    return this.request<PriceQuote>(path, signal);
  }

  /**
   * Get a quote and validate it is not stale or expired.
   * Throws {@link StellarRouteApiError} with code `"quote_expired"` or
   * `"quote_stale"` when the quote fails the staleness check.
   */
  async getQuoteWithValidation(
    base: string,
    quote: string,
    amount?: number,
    type: QuoteType = 'sell',
    slippage?: number,
    stalenessConfig: QuoteStalenessConfig = DEFAULT_STALENESS_CONFIG,
    signal?: AbortSignal,
  ): Promise<PriceQuote> {
    const quoteResponse = await this.getQuote(base, quote, amount, type, slippage, signal);

    if (isQuoteExpired(quoteResponse)) {
      throw new StellarRouteApiError(
        0,
        'quote_expired',
        'Quote has expired based on server-provided expiry time',
        { expires_at: quoteResponse.expires_at },
      );
    }

    if (stalenessConfig.reject_stale && isQuoteStale(quoteResponse, stalenessConfig)) {
      throw new StellarRouteApiError(
        0,
        'quote_stale',
        `Quote is stale (older than ${stalenessConfig.max_age_seconds} seconds)`,
        { timestamp: quoteResponse.timestamp, max_age_seconds: stalenessConfig.max_age_seconds },
      );
    }

    return quoteResponse;
  }

  /**
   * `GET /api/v1/route/{base}/{quote}` — get optimal trading route.
   *
   * @deprecated Use {@link getRankedRoutes} instead, which calls the ranked
   *   `/api/v1/routes` endpoint and returns richer candidate data including
   *   scores, feasibility fields, and multiple route options.
   *
   * @param base   Base asset identifier.
   * @param quote  Quote asset identifier.
   * @param amount Amount of the base asset to trade.
   * @param type   Direction of the quote (`"sell"` or `"buy"`).
   * @param slippage Slippage tolerance in basis points.
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when no route exists.
   */
  async getRoutes(
    base: string,
    quote: string,
    amount?: number,
    type: QuoteType = 'sell',
    slippage?: number,
    signal?: AbortSignal,
  ): Promise<PathStep[]> {
    const params = new URLSearchParams({ quote_type: type });
    if (amount !== undefined) params.set('amount', String(amount));
    if (slippage !== undefined) params.set('slippage_bps', String(slippage));

    const path = `/api/v1/route/${encodeURIComponent(base)}/${encodeURIComponent(quote)}?${params}`;
    const response = await this.request<RouteResponse>(path, signal);
    return response.path;
  }

  /**
   * `GET /api/v1/routes/{base}/{quote}` — get ranked trading route candidates.
   *
   * Returns multiple ranked routes with composite scores, price impact, and
   * per-hop metadata. Routes are sorted by score (higher is better).
   *
   * @param base    Base asset identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`.
   * @param quote   Quote asset identifier.
   * @param amount  Amount of the base asset to trade. Defaults to `"1"`.
   * @param limit   Maximum number of routes to return (1–20, default 5).
   * @param maxHops Maximum hops per route (1–6, default 3).
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when no route exists.
   * @throws {@link StellarRouteApiError} with `status === 400` for invalid params.
   */
  async getRankedRoutes(
    base: string,
    quote: string,
    amount?: number,
    limit?: number,
    maxHops?: number,
    signal?: AbortSignal,
  ): Promise<RankedRoutesResponse> {
    const params = new URLSearchParams();
    if (amount !== undefined) params.set('amount', String(amount));
    if (limit !== undefined) params.set('limit', String(limit));
    if (maxHops !== undefined) params.set('max_hops', String(maxHops));

    const qs = params.toString();
    const path = `/api/v1/routes/${encodeURIComponent(base)}/${encodeURIComponent(quote)}${qs ? `?${qs}` : ''}`;
    return this.request<RankedRoutesResponse>(path, signal);
  }

  /**
   * `POST /api/v1/batch/quote` — fetch multiple price quotes in a single request.
   *
   * @param requests Array of quote requests to fetch.
   *
   * @throws {@link StellarRouteApiError} when the batch request fails.
   */
  async getQuotesBatch(
    requests: QuoteRequestItem[],
    signal?: AbortSignal,
  ): Promise<BatchQuoteResponse> {
    const path = '/api/v1/batch/quote';
    return this.request<BatchQuoteResponse>(
      path,
      signal,
      this.retries,
      'POST',
      { quotes: requests },
    );
  }

  /**
   * `POST /api/v1/simulate/route` — dry-run a route to see expected output,
   * price impact, and slippage before committing to an on-chain swap.
   *
   * @param params Route and simulation parameters.
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when no
   *   matching route exists.
   * @throws {@link StellarRouteApiError} with `status === 400` for invalid
   *   request params.
   *
   * @example
   * ```ts
   * const result = await client.simulateRoute({
   *   route: { hops: [{ from_asset: 'native', to_asset: 'USDC:GA...', source: 'sdex' }] },
   *   amount: '100',
   *   slippage_bps: 50,
   * });
   * console.log(result.quote.price);
   * ```
   */
  simulateRoute(
    params: SimulateRouteRequest,
    signal?: AbortSignal,
  ): Promise<SimulateRouteResponse> {
    return this.request<SimulateRouteResponse>(
      '/api/v1/simulate/route',
      signal,
      this.retries,
      'POST',
      params,
    );
  }

  /**
   * `POST /api/v1/swap/prepare` — build an unsigned classic PathPaymentStrictSend.
   */
  async prepareSwap(
    params: SwapPrepareRequest,
    signal?: AbortSignal,
  ): Promise<PreparedSwapResponse> {
    const body = await this.request<unknown>(
      '/api/v1/swap/prepare',
      signal,
      this.retries,
      'POST',
      params,
    );
    const prepared = unwrapApiData<PreparedSwapResponse>(body);
    if (!prepared?.xdr_envelope || typeof prepared.xdr_envelope !== 'string') {
      throw new StellarRouteApiError(
        500,
        'internal_error',
        'Prepare response missing xdr_envelope',
      );
    }
    return prepared;
  }

  /**
   * `POST /api/v1/swap/submit` — broadcast a wallet-signed envelope.
   *
   * Pass `retries: 0` from {@link executeSwap} and use explicit ambiguous
   * retries so the same signed body is reused without re-prepare/re-sign.
   */
  async submitSwap(
    params: SwapSubmitRequest,
    signal?: AbortSignal,
    retries: number = this.retries,
  ): Promise<SwapSubmitResponse> {
    const body = await this.request<unknown>(
      '/api/v1/swap/submit',
      signal,
      retries,
      'POST',
      params,
    );
    return unwrapApiData<SwapSubmitResponse>(body);
  }

  /**
   * Confirm a submitted swap on Horizon (`GET /transactions/{tx_hash}`).
   */
  async confirmSwap(
    txHash: string,
    options?: {
      horizonUrl?: string;
      timeoutMs?: number;
      pollIntervalMs?: number;
      signal?: AbortSignal;
      expectedTxHash?: string;
    },
  ): Promise<SwapConfirmResult> {
    const expected = options?.expectedTxHash ?? txHash;
    const horizonUrl = (options?.horizonUrl ?? DEFAULT_TESTNET_HORIZON_URL).replace(
      /\/$/,
      '',
    );
    const timeoutMs = options?.timeoutMs ?? 60_000;
    const pollIntervalMs = options?.pollIntervalMs ?? 2_000;
    const url = `${horizonUrl}/transactions/${encodeURIComponent(expected)}`;
    const deadline = Date.now() + timeoutMs;

    while (Date.now() <= deadline) {
      if (options?.signal?.aborted) {
        throw new StellarRouteApiError(0, 'network_error', 'confirmSwap aborted');
      }

      let response: Response;
      try {
        response = await fetch(url, {
          method: 'GET',
          headers: { Accept: 'application/json' },
          signal: options?.signal,
        });
      } catch (err) {
        if (Date.now() + pollIntervalMs > deadline) {
          const message = err instanceof Error ? err.message : 'Network error';
          throw new StellarRouteApiError(0, 'network_error', message);
        }
        await sleep(pollIntervalMs);
        continue;
      }

      if (response.status === 404) {
        if (Date.now() + pollIntervalMs > deadline) break;
        await sleep(pollIntervalMs);
        continue;
      }

      if (!response.ok) {
        throw new StellarRouteApiError(
          response.status,
          'unknown_error',
          `Horizon confirmation failed with HTTP ${response.status}`,
        );
      }

      const body = (await response.json()) as {
        hash?: string;
        successful?: boolean;
        ledger?: number;
      };
      const hash = body.hash ?? expected;
      if (hash !== expected) {
        throw new StellarRouteApiError(
          500,
          'internal_error',
          `Horizon hash mismatch: expected ${expected}, got ${hash}`,
        );
      }

      return {
        tx_hash: hash,
        successful: Boolean(body.successful),
        ledger: body.ledger,
        horizon_url: url,
      };
    }

    throw new StellarRouteApiError(
      0,
      'network_error',
      `Transaction ${expected} not confirmed on Horizon within ${timeoutMs}ms`,
      { horizon_url: url, status: 'confirm_timeout' },
    );
  }

  /**
   * Execute a classic swap: prepare → network check → sign once → submit
   * (with ambiguous retry).
   *
   * Requires `prepared.execution_mode === 'classic_path_payment'` and a non-empty
   * server `xdr_envelope`. Compares {@link ExecuteSwapParams.networkPassphrase}
   * to `prepared.network_passphrase` before signing; mismatch throws
   * `network_mismatch` without signing or submitting. Ambiguous submit
   * failures retry the **same** `{ quote_id, signed_xdr }` body without
   * re-prepare or re-sign.
   */
  async executeSwap(
    params: ExecuteSwapParams,
    signal?: AbortSignal,
  ): Promise<ExecuteSwapResult> {
    const prepareResponse = await this.prepareSwap(
      {
        route: params.route,
        amount: params.amount,
        sender: params.sender,
        min_output: params.min_output,
        slippage_bps: params.slippage_bps,
      },
      signal,
    );

    if (prepareResponse.execution_mode !== CLASSIC_EXECUTION_MODE) {
      throw new StellarRouteApiError(
        422,
        'unsupported_execution_mode',
        `Unsupported execution_mode '${prepareResponse.execution_mode}'; expected '${CLASSIC_EXECUTION_MODE}'`,
        { execution_mode: prepareResponse.execution_mode },
      );
    }

    if (!prepareResponse.xdr_envelope.trim()) {
      throw new StellarRouteApiError(
        500,
        'internal_error',
        'Prepare returned an empty xdr_envelope',
      );
    }

    const preparedPassphrase = prepareResponse.network_passphrase?.trim() ?? '';
    if (!preparedPassphrase) {
      throw new StellarRouteApiError(
        400,
        'validation_error',
        'Prepare returned an empty network_passphrase',
        { status: 'missing_network_passphrase' },
      );
    }

    const integratorRaw =
      typeof params.networkPassphrase === 'function'
        ? await params.networkPassphrase()
        : params.networkPassphrase;
    const integratorPassphrase =
      typeof integratorRaw === 'string' ? integratorRaw.trim() : '';
    if (!integratorPassphrase || integratorPassphrase !== preparedPassphrase) {
      throw new StellarRouteApiError(
        400,
        'network_mismatch',
        'Wallet/app network passphrase does not match the prepared swap network',
        {
          status: 'network_mismatch',
          prepared_network_passphrase: preparedPassphrase,
          integrator_network_passphrase: integratorPassphrase || null,
        },
      );
    }

    const signedXdr = await params.signTransaction(prepareResponse.xdr_envelope);
    if (!signedXdr || typeof signedXdr !== 'string' || !signedXdr.trim()) {
      throw new StellarRouteApiError(
        400,
        'validation_error',
        'signTransaction returned an empty signed envelope',
      );
    }

    const submitBody: SwapSubmitRequest = {
      quote_id: prepareResponse.quote_id,
      signed_xdr: signedXdr,
    };

    const maxAmbiguous = params.ambiguousSubmitRetries ?? 2;
    let submitResponse: SwapSubmitResponse | undefined;
    let lastErr: StellarRouteApiError | undefined;

    for (let attempt = 0; attempt <= maxAmbiguous; attempt++) {
      try {
        // retries: 0 — ambiguous handling is explicit below with the same body.
        submitResponse = await this.submitSwap(submitBody, signal, 0);
        lastErr = undefined;
        break;
      } catch (err) {
        if (!isStellarRouteApiError(err) || !isAmbiguousSubmitError(err)) {
          throw err;
        }
        lastErr = err;
        if (attempt >= maxAmbiguous) break;
        await sleep(backoffMs(attempt));
      }
    }

    if (!submitResponse) {
      throw new StellarRouteApiError(
        lastErr?.status ?? 503,
        lastErr?.code ?? 'dependency_unavailable',
        lastErr?.message ??
          'Submit is still pending; reconcile the bound quote before preparing again',
        {
          ...(typeof lastErr?.details === 'object' && lastErr.details
            ? (lastErr.details as object)
            : {}),
          status: 'pending_reconcile',
          quote_id: prepareResponse.quote_id,
        },
      );
    }

    return {
      quote_id: prepareResponse.quote_id,
      xdr_envelope: prepareResponse.xdr_envelope,
      expected_output: prepareResponse.expected_output,
      min_output: prepareResponse.min_output,
      expires_at: prepareResponse.expires_at,
      execution_mode: prepareResponse.execution_mode,
      network_passphrase: prepareResponse.network_passphrase,
      tx_hash: submitResponse.tx_hash,
      status: submitResponse.status,
    };
  }

  /**
   * `GET /api/v1/price-history/{base}/{quote}` — fetch price history for charting/sparklines.
   *
   * @param base  Base asset identifier: `"native"`, `"CODE"`, or `"CODE:ISSUER"`.
   * @param quote Quote asset identifier.
   * @param options Optional parameters: `window` for the time range.
   *
   * @throws {@link StellarRouteApiError} with `status === 404` when the pair is not found.
   */
  getPriceHistory(
    base: string,
    quote: string,
    options?: { window?: PriceHistoryWindow; signal?: AbortSignal },
  ): Promise<PriceHistoryResponse> {
    const params = new URLSearchParams();
    if (options?.window !== undefined) params.set('window', options.window);

    const qs = params.toString();
    const path = `/api/v1/price-history/${encodeURIComponent(base)}/${encodeURIComponent(quote)}${qs ? `?${qs}` : ''}`;
    return this.request<PriceHistoryResponse>(path, options?.signal);
  }

  /**
   * `GET /api/v2` — capability descriptor including CCTP corridor metadata.
   */
  async getApiV2Info(signal?: AbortSignal): Promise<ApiV2Info> {
    const body = await this.request<unknown>('/api/v2', signal);
    return unwrapApiData<ApiV2Info>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/quote` — CCTP fee quote (fail-closed until enabled).
   */
  async cctpQuote(
    request: CctpQuoteRequest,
    options?: CctpCallOptions,
  ): Promise<CctpQuoteResponse> {
    const headers: Record<string, string> = {};
    if (options?.idempotencyKey) {
      headers[CCTP_IDEMPOTENCY_HEADER] = options.idempotencyKey;
    }
    const body = await this.request<unknown>(
      '/api/v2/bridge/cctp/quote',
      options?.signal,
      this.retries,
      'POST',
      request,
      headers,
    );
    return unwrapApiData<CctpQuoteResponse>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/{transfer_id}/prepare-burn`
   */
  async cctpPrepareBurn(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpPrepareBurnResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}/prepare-burn`,
      options?.signal,
      this.retries,
      'POST',
      {},
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpPrepareBurnResponse>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/{transfer_id}/submit-burn` — tx hash acknowledgement only.
   */
  async cctpSubmitBurn(
    transferId: string,
    request: CctpSubmitBurnRequest,
    options?: CctpCallOptions,
  ): Promise<CctpSubmitBurnResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}/submit-burn`,
      options?.signal,
      this.retries,
      'POST',
      request,
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpSubmitBurnResponse>(body);
  }

  /**
   * `GET /api/v2/bridge/cctp/{transfer_id}` — transfer saga status.
   */
  async cctpGetTransfer(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpTransferStatusResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}`,
      options?.signal,
      this.retries,
      'GET',
      undefined,
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpTransferStatusResponse>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/{transfer_id}/prepare-mint`
   */
  async cctpPrepareMint(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpPrepareMintResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}/prepare-mint`,
      options?.signal,
      this.retries,
      'POST',
      {},
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpPrepareMintResponse>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/{transfer_id}/submit-mint` — tx hash acknowledgement only.
   */
  async cctpSubmitMint(
    transferId: string,
    request: CctpSubmitMintRequest,
    options?: CctpCallOptions,
  ): Promise<CctpSubmitMintResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}/submit-mint`,
      options?.signal,
      this.retries,
      'POST',
      request,
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpSubmitMintResponse>(body);
  }

  /**
   * `POST /api/v2/bridge/cctp/{transfer_id}/reattest`
   */
  async cctpReattest(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpReattestResponse> {
    const body = await this.request<unknown>(
      `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}/reattest`,
      options?.signal,
      this.retries,
      'POST',
      {},
      cctpAccessHeaders(options),
    );
    return unwrapApiData<CctpReattestResponse>(body);
  }

  // ── Internal helpers ────────────────────────────────────────────────────────

  private async request<T>(
    path: string,
    signal?: AbortSignal,
    attemptsLeft = this.retries,
    method: 'GET' | 'POST' = 'GET',
    body?: unknown,
    extraHeaders?: Record<string, string>,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    // Forward external cancellation into our controller.
    signal?.addEventListener('abort', () => controller.abort(), { once: true });

    try {
      const fetchOptions: RequestInit = {
        method,
        headers: {
          Accept: 'application/json',
          ...this.extraHeaders,
          ...extraHeaders,
        },
        signal: controller.signal,
      };

      if (body) {
        fetchOptions.body = JSON.stringify(body);
        (fetchOptions.headers as Record<string, string>)['Content-Type'] =
          'application/json';
      }

      const response = await fetch(url, fetchOptions);

      if (!response.ok) {
        // Parse flat or envelope-wrapped `{ data: { error, message, details } }`.
        let code: ApiErrorCode = 'unknown_error';
        let message = `HTTP ${response.status}`;
        let details: unknown;

        try {
          const parsed = parseApiErrorBody(await response.json());
          if (parsed.error) code = parsed.error as ApiErrorCode;
          if (parsed.message) message = parsed.message;
          details = parsed.details;
        } catch {
          // Non-JSON body — keep defaults.
        }

        // Retry on 429 and 5xx (same request body — safe for idempotent submits).
        // Do not retry 409 conflicts.
        if (
          response.status !== 409 &&
          (response.status === 429 || response.status >= 500) &&
          attemptsLeft > 0
        ) {
          const retryAfterSec = Number(response.headers.get('Retry-After') ?? 0);
          const delayMs = retryAfterSec > 0
            ? retryAfterSec * 1_000
            : backoffMs(this.retries - attemptsLeft);
          await sleep(delayMs);
          return this.request<T>(path, signal, attemptsLeft - 1, method, body, extraHeaders);
        }

        throw new StellarRouteApiError(response.status, code, message, details);
      }

      return response.json() as Promise<T>;
    } catch (err) {
      if (err instanceof StellarRouteApiError) throw err;

      // Retry on network errors.
      if (attemptsLeft > 0) {
        await sleep(backoffMs(this.retries - attemptsLeft));
        return this.request<T>(path, signal, attemptsLeft - 1, method, body);
      }

      const message = err instanceof Error ? err.message : 'Network error';
      throw new StellarRouteApiError(0, 'network_error', message);
    } finally {
      clearTimeout(timer);
    }
  }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

function cctpAccessHeaders(
  options?: CctpCallOptions,
): Record<string, string> | undefined {
  if (!options?.accessToken) return undefined;
  return { [CCTP_TRANSFER_ACCESS_HEADER]: options.accessToken };
}

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

/** Exponential back-off: 500 ms, 1 s, 2 s, … */
const backoffMs = (attempt: number): number => 500 * Math.pow(2, attempt);