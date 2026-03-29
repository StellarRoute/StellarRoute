import type {
  ApiErrorCode,
  HealthStatus,
  Orderbook,
  PairsResponse,
  PathStep,
  PriceQuote,
  QuoteRequestItem,
  BatchQuoteResponse,
  QuoteStalenessConfig,
  QuoteType,
  RouteResponse,
} from './types.js';
import { DEFAULT_STALENESS_CONFIG, isQuoteStale, isQuoteExpired } from './types.js';

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL = 'http://localhost:8080';
const DEFAULT_TIMEOUT_MS = 10_000;
const DEFAULT_RETRIES = 2;

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
      requests,
    );
  }

  // ── Internal helpers ────────────────────────────────────────────────────────

  private async request<T>(
    path: string,
    signal?: AbortSignal,
    attemptsLeft = this.retries,
    method: 'GET' | 'POST' = 'GET',
    body?: unknown,
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
        // Parse the structured error body when available.
        let code: ApiErrorCode = 'unknown_error';
        let message = `HTTP ${response.status}`;
        let details: unknown;

        try {
          const body = (await response.json()) as {
            error?: string;
            message?: string;
            details?: unknown;
          };
          if (body.error) code = body.error as ApiErrorCode;
          if (body.message) message = body.message;
          details = body.details;
        } catch {
          // Non-JSON body — keep defaults.
        }

        // Retry on 429 and 5xx.
        if ((response.status === 429 || response.status >= 500) && attemptsLeft > 0) {
          const retryAfterSec = Number(response.headers.get('Retry-After') ?? 0);
          const delayMs = retryAfterSec > 0
            ? retryAfterSec * 1_000
            : backoffMs(this.retries - attemptsLeft);
          await sleep(delayMs);
          return this.request<T>(path, signal, attemptsLeft - 1, method, body);
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

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

/** Exponential back-off: 500 ms, 1 s, 2 s, … */
const backoffMs = (attempt: number): number => 500 * Math.pow(2, attempt);
