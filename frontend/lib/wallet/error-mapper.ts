/**
 * Maps raw wallet errors to user-facing copy.
 *
 * Covers Freighter's explicit rejection codes as well as the other Stellar
 * wallets (xBull, Albedo, LOBSTR) that surface similar strings.  EVM adapter
 * errors (WalletAdapterError / EIP-1193 codes) are handled via
 * `normalizeProviderError` in `./adapters/errors.ts`; this module is for the
 * Stellar connection and signing layer.
 */

import type { WalletError } from './types';

export interface WalletErrorCopy {
  /** Short headline shown near the wallet button or in a toast. */
  headline: string;
  /** One-sentence explanation of what happened. */
  detail: string;
  /** Suggested recovery step for the user. */
  recovery: string;
}

/**
 * Freighter surfaces these exact strings when the user dismisses the popup
 * or the extension rejects access programmatically.
 */
const FREIGHTER_REJECTION_PHRASES = [
  'user declined',
  'declined access',
  'signing denied',
  'user rejected',
  'transaction was rejected',
  'request rejected',
  'user cancelled',
  'user canceled',
  'user cancel',
  'rejected by user',
  'denied by user',
  'user denied',
] as const;

/** EIP-1193 numeric/string codes that mean the user rejected. */
function isEip1193Rejection(raw: unknown): boolean {
  if (raw && typeof raw === 'object' && 'code' in raw) {
    const code = (raw as { code?: number | string }).code;
    if (code === 4001 || code === 'ACTION_REJECTED') return true;
  }
  return false;
}

function messageContains(lower: string, phrases: readonly string[]): boolean {
  return phrases.some((p) => lower.includes(p));
}

/**
 * Returns structured copy for a `WalletError` (or any `Error`-shaped object),
 * or `null` when no specific mapping applies (caller should fall back to
 * `error.message`).
 */
export function mapWalletError(
  error: WalletError | Error | unknown,
  walletId?: string | null,
): WalletErrorCopy | null {
  if (!error) return null;

  const walletLabel = resolveWalletLabel(walletId);

  // 1. Typed WalletError with an explicit code (set by the provider layer).
  if (isWalletError(error) && error.code) {
    return mapByCode(error.code, walletLabel);
  }

  // 2. EIP-1193 object shape.
  if (isEip1193Rejection(error)) {
    return rejectionCopy(walletLabel);
  }

  // 3. String message analysis.
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'object' && error !== null && 'message' in error
        ? String((error as { message: unknown }).message)
        : typeof error === 'string'
          ? error
          : '';

  const lower = message.toLowerCase();

  if (messageContains(lower, FREIGHTER_REJECTION_PHRASES)) {
    return rejectionCopy(walletLabel);
  }

  if (lower.includes('not installed') || lower.includes('extension not found')) {
    return {
      headline: `${walletLabel} is not installed`,
      detail: `The ${walletLabel} browser extension could not be found.`,
      recovery: `Install ${walletLabel} from the browser extension store, then try again.`,
    };
  }

  if (lower.includes('network') && (lower.includes('mismatch') || lower.includes('wrong'))) {
    return {
      headline: 'Network mismatch',
      detail: `${walletLabel} is connected to a different Stellar network than the app.`,
      recovery: 'Switch your wallet to the same network as the app and reconnect.',
    };
  }

  if (lower.includes('not connected') || lower.includes('no session')) {
    return {
      headline: 'Wallet not connected',
      detail: `${walletLabel} is no longer connected.`,
      recovery: 'Reconnect your wallet to continue.',
    };
  }

  return null;
}

function mapByCode(code: string, walletLabel: string): WalletErrorCopy | null {
  switch (code) {
    case 'user_rejected':
      return rejectionCopy(walletLabel);
    case 'not_installed':
      return {
        headline: `${walletLabel} is not installed`,
        detail: `The ${walletLabel} browser extension could not be found.`,
        recovery: `Install ${walletLabel} from the browser extension store, then try again.`,
      };
    case 'network_mismatch':
      return {
        headline: 'Network mismatch',
        detail: `${walletLabel} is connected to a different Stellar network than the app.`,
        recovery: 'Switch your wallet network to match the app and reconnect.',
      };
    case 'not_connected':
      return {
        headline: 'Wallet not connected',
        detail: `${walletLabel} session ended or was revoked.`,
        recovery: 'Reconnect your wallet to continue.',
      };
    default:
      return null;
  }
}

function rejectionCopy(walletLabel: string): WalletErrorCopy {
  return {
    headline: 'Request declined',
    detail: `${walletLabel} did not approve the request.`,
    recovery: 'Open your wallet, approve the request, and try again.',
  };
}

function resolveWalletLabel(walletId?: string | null): string {
  switch (walletId) {
    case 'freighter':
      return 'Freighter';
    case 'xbull':
      return 'xBull';
    case 'albedo':
      return 'Albedo';
    case 'lobstr':
      return 'LOBSTR';
    default:
      return 'Your wallet';
  }
}

function isWalletError(value: unknown): value is WalletError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'message' in value &&
    typeof (value as WalletError).message === 'string'
  );
}
