/**
 * StellarRoute API client
 *
 * Single source of truth for all frontend-to-backend communication.
 * Covers every REST endpoint exposed by the StellarRoute backend.
 *
 * Base URL defaults to NEXT_PUBLIC_API_URL env var, falling back to
 * http://localhost:8080 (no /api/v1 suffix — paths are added per method).
 */

import type {
  HealthStatus,
  Orderbook,
  PairsResponse,
  PriceQuote,
  QuoteType,
  ApiErrorCode,
} from '@/types';

// ---------------------------------------------------------------------------
// Error class
// ---------------------------------------------------------------------------

export class StellarRouteApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: ApiErrorCode,
    message: string,
    public readonly details?: unknown,
  ) {
    super(message);
    this.name = 'StellarRouteApiError';
  }

  get isRateLimit(): boolean {
    return this.status === 429;
  }

  get isServerError(): boolean {
    return this.status >= 500;
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

const DEFAULT_TIMEOUT_MS = 10_000;

/** Sleep for `ms` milliseconds. */
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

interface FetchOptions {
  signal?: AbortSignal;
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

export class StellarRouteClient {
  private readonly baseUrl: string;
  private readonly retries: number = 2;

  constructor(baseUrl?: string) {
    this.baseUrl =
      (baseUrl ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8080')
        // Strip trailing slash so we can always prepend / paths uniformly
        .replace(/\/$/, '');
  }

  // -------------------------------------------------------------------------
  // Core fetch wrapper
  // -------------------------------------------------------------------------

  private async request<T>(
    path: string,
    opts: FetchOptions = {},
    retries = 2,
    method: 'GET' | 'POST' = 'GET',
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(
      () => controller.abort(),
      DEFAULT_TIMEOUT_MS,
    );

    // Honour an external AbortSignal as well
    opts.signal?.addEventListener('abort', () => controller.abort());

    try {
      const fetchOptions: RequestInit = {
        method,
        headers: { Accept: 'application/json' },
        signal: controller.signal,
      };

      if (body) {
        fetchOptions.body = JSON.stringify(body);
        (fetchOptions.headers as Record<string, string>)['Content-Type'] =
          'application/json';
      }

      const response = await fetch(url, fetchOptions);

      if (!response.ok) {
        // Try to parse the backend ErrorResponse body
        let code: ApiErrorCode = 'unknown_error';
        let message = `HTTP ${response.status}`;
        let details: unknown;

        try {
          const body = await response.json();
          code = (body.error as ApiErrorCode) ?? code;
          message = body.message ?? message;
          details = body.details;
        } catch {
          // Body was not JSON — keep defaults
        }

        // Retry on rate-limit (429) and server errors (5xx) with backoff
        if ((response.status === 429 || response.status >= 500) && retries > 0) {
          const retryAfter =
            Number(response.headers.get('Retry-After') ?? 1) * 1_000;
          await sleep(retryAfter || 1_000 * (3 - retries));
          return this.request<T>(path, opts, retries - 1, method, body);
        }

        throw new StellarRouteApiError(response.status, code, message, details);
      }

      return response.json() as Promise<T>;
    } catch (err) {
      if (err instanceof StellarRouteApiError) throw err;

      // Network error / timeout
      if (retries > 0) {
        await sleep(500 * (3 - retries));
        return this.request<T>(path, opts, retries - 1, method, body);
      }

      const message =
        err instanceof Error ? err.message : 'Network error';
      throw new StellarRouteApiError(0, 'network_error' as ApiErrorCode, message);
    } finally {
      clearTimeout(timer);
    }
  }

  // -------------------------------------------------------------------------
  // Public API methods
  // -------------------------------------------------------------------------

  /** GET /health — overall service health check */
  getHealth(opts?: FetchOptions): Promise<HealthStatus> {
    return this.request<HealthStatus>('/health', opts);
  }

  /** GET /api/v1/pairs — list all trading pairs */
  getPairs(opts?: FetchOptions): Promise<PairsResponse> {
    return this.request<PairsResponse>('/api/v1/pairs', opts);
  }

  /**
   * GET /api/v1/orderbook/{base}/{quote}
   *
   * @param base  Asset identifier: "native" | "CODE" | "CODE:ISSUER"
   * @param quote Asset identifier: "native" | "CODE" | "CODE:ISSUER"
   */
  getOrderbook(
    base: string,
    quote: string,
    opts?: FetchOptions,
  ): Promise<Orderbook> {
    const path = `/api/v1/orderbook/${encodeURIComponent(base)}/${encodeURIComponent(quote)}`;
    return this.request<Orderbook>(path, opts);
  }

  /**
   * GET /api/v1/quote/{base}/{quote}?amount={amount}&quote_type={sell|buy}
   *
   * @param base   Asset identifier
   * @param quote  Asset identifier
   * @param amount Amount to trade (optional)
   * @param type   "sell" (default) or "buy"
   */
  getQuote(
    base: string,
    quote: string,
    amount?: number,
    type: QuoteType = 'sell',
    opts?: FetchOptions,
  ): Promise<PriceQuote> {
    const params = new URLSearchParams({ quote_type: type });
    if (amount !== undefined) params.set('amount', String(amount));
    const path = `/api/v1/quote/${encodeURIComponent(base)}/${encodeURIComponent(quote)}?${params}`;
    return this.request<PriceQuote>(path, opts);
  }

  /**
   * POST /api/v1/batch/quote — fetch multiple price quotes in a single request.
   *
   * @param requests Array of quote requests to fetch.
   *
   * @throws {StellarRouteApiError} when the batch request fails.
   */
  async getQuotesBatch(
    requests: QuoteRequestItem[],
    opts?: FetchOptions,
  ): Promise<BatchQuoteResponse> {
    const path = '/api/v1/batch/quote';
    return this.request<BatchQuoteResponse>(
      path,
      opts,
      this.retries,
      'POST',
      requests,
    );
  }
}

/** Single request item for a batch quote. */
export interface QuoteRequestItem {
  base: string;
  quote: string;
  amount?: number;
  quote_type?: QuoteType;
}

/** Response from a batch quote request. */
export interface BatchQuoteResponse {
  quotes: PriceQuote[];
  total: number;
}

// ---------------------------------------------------------------------------
// Singleton — use this in hooks and server components
// ---------------------------------------------------------------------------

export const stellarRouteClient = new StellarRouteClient();
