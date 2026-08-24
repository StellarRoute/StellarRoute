import type { QuoteType } from '@/types';

export const QUOTE_RETRY_EVENT_NAME = 'stellarroute:quote-retry';

export interface QuoteRetryBackoffOptions {
  baseDelayMs: number;
  maxDelayMs: number;
  jitterRatio: number;
}

export interface QuoteRetryRequestContext {
  base: string;
  quoteAsset: string;
  amount: number;
  type: QuoteType;
}

export interface QuoteRetryTelemetryEvent {
  stage: 'scheduled' | 'cancelled' | 'succeeded' | 'failed';
  request: QuoteRetryRequestContext;
  attempt: number;
  delayMs: number;
  errorMessage?: string;
}

export function getQuoteRetryRequestKey(
  request: QuoteRetryRequestContext,
): string {
  return [request.base, request.quoteAsset, request.amount, request.type].join(
    '|',
  );
}

export function calculateQuoteRetryDelayMs(
  attempt: number,
  options: QuoteRetryBackoffOptions,
  random = Math.random,
): number {
  const sanitizedAttempt = Math.max(1, attempt);
  const exponentialDelay = Math.min(
    options.baseDelayMs * 2 ** (sanitizedAttempt - 1),
    options.maxDelayMs,
  );
  const boundedJitterRatio = Math.min(Math.max(options.jitterRatio, 0), 1);

  if (boundedJitterRatio === 0) {
    return exponentialDelay;
  }

  const jitterSpan = exponentialDelay * boundedJitterRatio;
  const jitterOffset = (random() * 2 - 1) * jitterSpan;
  return Math.max(0, Math.round(exponentialDelay + jitterOffset));
}

export function emitQuoteRetryTelemetry(
  event: QuoteRetryTelemetryEvent,
  onEvent?: (event: QuoteRetryTelemetryEvent) => void,
): void {
  onEvent?.(event);

  if (typeof window === 'undefined' || typeof CustomEvent === 'undefined') {
    return;
  }

  window.dispatchEvent(
    new CustomEvent<QuoteRetryTelemetryEvent>(QUOTE_RETRY_EVENT_NAME, {
      detail: event,
    }),
  );
}
