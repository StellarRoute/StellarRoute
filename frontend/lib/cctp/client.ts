import { StellarRouteApiError } from '@/lib/api/client';
import { getApiRoot } from '@/lib/constants';
import type {
  ApiV2Info,
  CctpCallOptions,
  CctpPrepareBurnResponse,
  CctpPrepareMintResponse,
  CctpQuoteRequest,
  CctpQuoteResponse,
  CctpReattestResponse,
  CctpSubmitBurnRequest,
  CctpSubmitBurnResponse,
  CctpSubmitMintRequest,
  CctpSubmitMintResponse,
  CctpTransferStatusResponse,
} from './types';
import {
  CCTP_IDEMPOTENCY_HEADER,
  CCTP_TRANSFER_ACCESS_HEADER,
} from './types';

const DEFAULT_TIMEOUT_MS = 15_000;
const QUOTE_425_MAX_ATTEMPTS = 6;
const QUOTE_425_BASE_MS = 400;

function unwrapEnvelope<T>(body: unknown): T {
  if (body && typeof body === 'object' && 'data' in body) {
    return (body as { data: T }).data;
  }
  return body as T;
}

function parseErrorBody(body: unknown): {
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
      ? (root.data as { error?: unknown; message?: unknown; details?: unknown })
      : null;
  const source =
    typeof root.error === 'string'
      ? root
      : nested && typeof nested.error === 'string'
        ? nested
        : nested ?? root;
  return {
    error: typeof source.error === 'string' ? source.error : undefined,
    message: typeof source.message === 'string' ? source.message : undefined,
    details: source.details,
  };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function jitteredBackoff(attempt: number): number {
  const base = QUOTE_425_BASE_MS * Math.pow(2, attempt);
  return base + Math.floor(Math.random() * 150);
}

export class CctpApiClient {
  private readonly baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = (baseUrl ?? getApiRoot()).replace(/\/$/, '');
  }

  async getApiV2Info(signal?: AbortSignal): Promise<ApiV2Info> {
    const body = await this.request<unknown>('/api/v2', { signal });
    return unwrapEnvelope<ApiV2Info>(body);
  }

  async quote(
    request: CctpQuoteRequest,
    options?: CctpCallOptions,
  ): Promise<CctpQuoteResponse> {
    return this.quoteWith425Retry(request, options);
  }

  private async quoteWith425Retry(
    request: CctpQuoteRequest,
    options?: CctpCallOptions,
  ): Promise<CctpQuoteResponse> {
    let attempt = 0;
    while (true) {
      if (options?.signal?.aborted) {
        throw new StellarRouteApiError(0, 'network_error', 'Quote cancelled');
      }
      try {
        const body = await this.request<unknown>(
          '/api/v2/bridge/cctp/quote',
          {
            method: 'POST',
            body: request,
            headers: cctpHeaders(options),
            signal: options?.signal,
            retries: 0,
          },
        );
        return unwrapEnvelope<CctpQuoteResponse>(body);
      } catch (err) {
        if (
          err instanceof StellarRouteApiError &&
          err.status === 425 &&
          attempt < QUOTE_425_MAX_ATTEMPTS
        ) {
          await sleep(jitteredBackoff(attempt));
          attempt += 1;
          continue;
        }
        throw err;
      }
    }
  }

  prepareBurn(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpPrepareBurnResponse> {
    return this.postTransfer<CctpPrepareBurnResponse>(
      transferId,
      'prepare-burn',
      {},
      options,
    );
  }

  submitBurn(
    transferId: string,
    request: CctpSubmitBurnRequest,
    options?: CctpCallOptions,
  ): Promise<CctpSubmitBurnResponse> {
    return this.postTransfer<CctpSubmitBurnResponse>(
      transferId,
      'submit-burn',
      request,
      options,
    );
  }

  getTransfer(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpTransferStatusResponse> {
    return this.requestTransfer<CctpTransferStatusResponse>(
      transferId,
      '',
      { method: 'GET', headers: cctpHeaders(options), signal: options?.signal },
    );
  }

  prepareMint(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpPrepareMintResponse> {
    return this.postTransfer<CctpPrepareMintResponse>(
      transferId,
      'prepare-mint',
      {},
      options,
    );
  }

  submitMint(
    transferId: string,
    request: CctpSubmitMintRequest,
    options?: CctpCallOptions,
  ): Promise<CctpSubmitMintResponse> {
    return this.postTransfer<CctpSubmitMintResponse>(
      transferId,
      'submit-mint',
      request,
      options,
    );
  }

  reattest(
    transferId: string,
    options?: CctpCallOptions,
  ): Promise<CctpReattestResponse> {
    return this.postTransfer<CctpReattestResponse>(
      transferId,
      'reattest',
      {},
      options,
    );
  }

  private postTransfer<T>(
    transferId: string,
    action: string,
    body: unknown,
    options?: CctpCallOptions,
  ): Promise<T> {
    return this.requestTransfer<T>(transferId, action, {
      method: 'POST',
      body,
      headers: cctpHeaders(options),
      signal: options?.signal,
    });
  }

  private async requestTransfer<T>(
    transferId: string,
    action: string,
    init: {
      method: 'GET' | 'POST';
      body?: unknown;
      headers?: Record<string, string>;
      signal?: AbortSignal;
    },
  ): Promise<T> {
    const suffix = action ? `/${action}` : '';
    const path = `/api/v2/bridge/cctp/${encodeURIComponent(transferId)}${suffix}`;
    const body = await this.request<unknown>(path, init);
    return unwrapEnvelope<T>(body);
  }

  private async request<T>(
    path: string,
    init: {
      method?: 'GET' | 'POST';
      body?: unknown;
      headers?: Record<string, string>;
      signal?: AbortSignal;
      retries?: number;
    } = {},
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
    init.signal?.addEventListener('abort', () => controller.abort(), {
      once: true,
    });

    const attemptsLeft = init.retries ?? 2;

    try {
      const headers: Record<string, string> = {
        Accept: 'application/json',
        ...init.headers,
      };
      const fetchInit: RequestInit = {
        method: init.method ?? 'GET',
        headers,
        signal: controller.signal,
      };
      if (init.body !== undefined) {
        fetchInit.body = JSON.stringify(init.body);
        headers['Content-Type'] = 'application/json';
      }

      const response = await fetch(url, fetchInit);
      if (!response.ok) {
        let code = 'unknown_error';
        let message = `HTTP ${response.status}`;
        let details: unknown;
        try {
          const parsed = parseErrorBody(await response.json());
          if (parsed.error) code = parsed.error;
          if (parsed.message) message = parsed.message;
          details = parsed.details;
        } catch {
          // non-json
        }

        if (
          response.status !== 409 &&
          response.status !== 425 &&
          (response.status === 429 || response.status >= 500) &&
          attemptsLeft > 0
        ) {
          await sleep(500 * (3 - attemptsLeft));
          return this.request<T>(path, {
            ...init,
            retries: attemptsLeft - 1,
          });
        }

        throw new StellarRouteApiError(
          response.status,
          code as never,
          message,
          details,
        );
      }

      return (await response.json()) as T;
    } catch (err) {
      if (err instanceof StellarRouteApiError) throw err;
      if (attemptsLeft > 0) {
        await sleep(500 * (3 - attemptsLeft));
        return this.request<T>(path, { ...init, retries: attemptsLeft - 1 });
      }
      const message = err instanceof Error ? err.message : 'Network error';
      throw new StellarRouteApiError(0, 'network_error', message);
    } finally {
      clearTimeout(timer);
    }
  }
}

function cctpHeaders(
  options?: CctpCallOptions,
): Record<string, string> | undefined {
  const headers: Record<string, string> = {};
  if (options?.accessToken) {
    headers[CCTP_TRANSFER_ACCESS_HEADER] = options.accessToken;
  }
  if (options?.idempotencyKey) {
    headers[CCTP_IDEMPOTENCY_HEADER] = options.idempotencyKey;
  }
  return Object.keys(headers).length > 0 ? headers : undefined;
}

let defaultClient: CctpApiClient | null = null;

export function getCctpApiClient(): CctpApiClient {
  if (!defaultClient) {
    defaultClient = new CctpApiClient();
  }
  return defaultClient;
}

export function resetCctpApiClientForTests(): void {
  defaultClient = null;
}
