import { StellarRouteApiError } from '@/lib/api/client';
import type { ApiErrorCode } from '@/types';

export interface TraderErrorCopy {
  headline: string;
  explanation: string;
  recoveryAction: string;
  ctaLabel: string;
}

const DEFAULT_COPY: TraderErrorCopy = {
  headline: 'We could not refresh this quote',
  explanation: 'Something unexpected happened while preparing your trade details.',
  recoveryAction: 'Refresh the quote, then try again.',
  ctaLabel: 'Refresh quote',
};

const API_ERROR_COPY: Record<ApiErrorCode, TraderErrorCopy> = {
  validation_error: {
    headline: 'Check your trade details',
    explanation: 'One or more inputs are outside the allowed format or range.',
    recoveryAction: 'Update the amount or pair, then refresh the quote.',
    ctaLabel: 'Review trade inputs',
  },
  invalid_asset: {
    headline: 'This asset pair is not available right now',
    explanation: 'The selected asset format or issuer could not be matched.',
    recoveryAction: 'Choose a supported asset pair and try again.',
    ctaLabel: 'Select another pair',
  },
  no_route: {
    headline: 'No executable route found',
    explanation: 'Current liquidity cannot complete this trade at the requested size.',
    recoveryAction: 'Try a smaller amount or a different pair.',
    ctaLabel: 'Adjust trade size',
  },
  stale_market_data: {
    headline: 'Market data is still updating',
    explanation: 'Fresh pricing is not available yet for this route.',
    recoveryAction: 'Wait a moment and refresh to fetch a current quote.',
    ctaLabel: 'Refresh in a few seconds',
  },
  rate_limit_exceeded: {
    headline: 'Quote refresh is temporarily limited',
    explanation: 'Too many quote requests were sent in a short window.',
    recoveryAction: 'Wait briefly before refreshing again.',
    ctaLabel: 'Try again shortly',
  },
  overloaded: {
    headline: 'Quote service is handling high traffic',
    explanation: 'Routing services are taking longer than normal to respond.',
    recoveryAction: 'Retry in a moment to request a fresh quote.',
    ctaLabel: 'Retry quote',
  },
  bad_request: {
    headline: 'We could not process this request',
    explanation: 'The quote request did not match the expected API format.',
    recoveryAction: 'Refresh and try again with updated trade inputs.',
    ctaLabel: 'Refresh quote',
  },
  unauthorized: {
    headline: 'Session check required',
    explanation: 'Your current request needs a valid session context.',
    recoveryAction: 'Reconnect wallet or reload the page, then retry.',
    ctaLabel: 'Reconnect wallet',
  },
  not_found: {
    headline: 'Requested market data was not found',
    explanation: 'The selected pair or route data is currently unavailable.',
    recoveryAction: 'Pick another pair and request a new quote.',
    ctaLabel: 'Choose another pair',
  },
  internal_error: {
    headline: 'Quote service hit an internal issue',
    explanation: 'The request reached the server but could not be completed safely.',
    recoveryAction: 'Retry shortly while we stabilize the route response.',
    ctaLabel: 'Retry quote',
  },
  network_error: {
    headline: 'Network connection interrupted',
    explanation: 'The app could not reach routing services from this device.',
    recoveryAction: 'Check your connection and refresh once online.',
    ctaLabel: 'Reconnect and refresh',
  },
  unknown_error: DEFAULT_COPY,
};

function inferWalletError(errorMessage: string): TraderErrorCopy | null {
  const text = errorMessage.toLowerCase();

  if (
    text.includes('wallet') ||
    text.includes('freighter') ||
    text.includes('xbull') ||
    text.includes('rejected') ||
    text.includes('denied') ||
    text.includes('signature')
  ) {
    return {
      headline: 'Wallet action was not completed',
      explanation: 'The wallet did not confirm the request needed to continue.',
      recoveryAction: 'Reopen your wallet, approve the request, and submit again.',
      ctaLabel: 'Open wallet and retry',
    };
  }

  return null;
}

function inferNetworkError(errorMessage: string): TraderErrorCopy | null {
  const text = errorMessage.toLowerCase();

  if (
    text.includes('network') ||
    text.includes('timeout') ||
    text.includes('failed to fetch') ||
    text.includes('offline')
  ) {
    return API_ERROR_COPY.network_error;
  }

  return null;
}

export function getTraderErrorCopy(error: unknown): TraderErrorCopy {
  if (error instanceof StellarRouteApiError) {
    if (error.code && error.code !== 'unknown_error' && API_ERROR_COPY[error.code]) {
      return API_ERROR_COPY[error.code];
    }
    
    if (error.status === 400) return API_ERROR_COPY.bad_request;
    if (error.status === 401) return API_ERROR_COPY.unauthorized;
    if (error.status === 404) return API_ERROR_COPY.not_found;
    if (error.status === 429) return API_ERROR_COPY.rate_limit_exceeded;
    if (error.status >= 500) return API_ERROR_COPY.internal_error;

    return API_ERROR_COPY[error.code] ?? DEFAULT_COPY;
  }

  if (error && typeof error === 'object' && 'status' in error && typeof (error as any).status === 'number') {
    const status = (error as any).status;
    if (status === 400) return API_ERROR_COPY.bad_request;
    if (status === 401) return API_ERROR_COPY.unauthorized;
    if (status === 404) return API_ERROR_COPY.not_found;
    if (status === 429) return API_ERROR_COPY.rate_limit_exceeded;
    if (status >= 500) return API_ERROR_COPY.internal_error;
  }

  if (error instanceof Error) {
    const walletCopy = inferWalletError(error.message);
    if (walletCopy) {
      return walletCopy;
    }

    const networkCopy = inferNetworkError(error.message);
    if (networkCopy) {
      return networkCopy;
    }
  }

  return DEFAULT_COPY;
}

export function toTraderErrorLine(copy: TraderErrorCopy): string {
  return `${copy.headline}. ${copy.explanation} ${copy.recoveryAction}`;
}
