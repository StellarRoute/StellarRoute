/**
 * Normalized wallet adapter errors — never include secrets.
 */

export type WalletAdapterErrorCode =
  | 'not_installed'
  | 'user_rejected'
  | 'network_mismatch'
  | 'unsupported_capability'
  | 'not_connected'
  | 'invalid_request'
  | 'provider_error';

export class WalletAdapterError extends Error {
  readonly code: WalletAdapterErrorCode;
  readonly adapterId?: string;

  constructor(
    message: string,
    code: WalletAdapterErrorCode,
    adapterId?: string
  ) {
    super(message);
    this.name = 'WalletAdapterError';
    this.code = code;
    this.adapterId = adapterId;
  }
}

/**
 * Explicit user-rejection phrases only.
 * Avoid bare `cancel` / `reject` / `declined` substrings (false positives).
 */
export function isUserRejection(message: string): boolean {
  const lower = message.toLowerCase();
  if (/\b4001\b/.test(lower) || lower.includes('action_rejected')) {
    return true;
  }
  const phrases = [
    'user rejected',
    'user denied',
    'user cancelled',
    'user canceled',
    'user cancel',
    'rejected by user',
    'denied by user',
    'user declined',
    'request rejected by user',
    'transaction was rejected by user',
  ];
  return phrases.some((phrase) => lower.includes(phrase));
}

export function isRpcMethodNotFound(err: unknown): boolean {
  if (err && typeof err === 'object' && 'code' in err) {
    const code = (err as { code?: number | string }).code;
    if (code === -32601 || code === 'METHOD_NOT_SUPPORTED') {
      return true;
    }
  }
  const message =
    err instanceof Error
      ? err.message
      : typeof err === 'string'
        ? err
        : '';
  const lower = message.toLowerCase();
  return (
    lower.includes('method not found') ||
    lower.includes('method not supported') ||
    lower.includes('does not exist / is not available') ||
    /eth_signtransaction.*(not supported|unsupported|unavailable)/i.test(
      message
    ) ||
    /the method ["']?eth_signtransaction["']? does not exist/i.test(message)
  );
}

export function normalizeProviderError(
  err: unknown,
  fallback: string,
  adapterId?: string
): WalletAdapterError {
  if (err instanceof WalletAdapterError) {
    return err;
  }

  const message =
    err instanceof Error
      ? err.message
      : typeof err === 'string'
        ? err
        : fallback;

  // EIP-1193 ProviderRpcError shape
  if (err && typeof err === 'object' && 'code' in err) {
    const code = (err as { code?: number | string }).code;
    if (code === 4001 || code === 'ACTION_REJECTED') {
      return new WalletAdapterError(
        'User rejected the wallet request',
        'user_rejected',
        adapterId
      );
    }
    if (code === 4902) {
      return new WalletAdapterError(
        'Wallet does not recognize the requested network',
        'network_mismatch',
        adapterId
      );
    }
  }

  if (isUserRejection(message)) {
    return new WalletAdapterError(
      'User rejected the wallet request',
      'user_rejected',
      adapterId
    );
  }

  return new WalletAdapterError(message || fallback, 'provider_error', adapterId);
}
