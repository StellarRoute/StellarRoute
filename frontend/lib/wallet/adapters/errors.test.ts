import { describe, expect, it } from 'vitest';
import { isUserRejection, normalizeProviderError } from './errors';

describe('isUserRejection', () => {
  it('matches explicit user-rejection phrases', () => {
    expect(isUserRejection('User rejected the request')).toBe(true);
    expect(isUserRejection('User denied transaction signature')).toBe(true);
    expect(isUserRejection('User cancelled the operation')).toBe(true);
    expect(isUserRejection('Request rejected by user')).toBe(true);
    expect(isUserRejection('ACTION_REJECTED')).toBe(true);
    expect(isUserRejection('error code 4001 from provider')).toBe(true);
  });

  it('does not false-positive on bare cancel/reject substrings', () => {
    expect(isUserRejection('Request cancelled by timeout')).toBe(false);
    expect(isUserRejection('RPC reject: insufficient funds')).toBe(false);
    expect(isUserRejection('cancel pending nonce')).toBe(false);
    expect(isUserRejection('reject rate limit')).toBe(false);
    expect(isUserRejection('declined by relay')).toBe(false);
    expect(isUserRejection('network error')).toBe(false);
  });
});

describe('normalizeProviderError', () => {
  it('maps EIP-1193 code 4001 to user_rejected', () => {
    const err = Object.assign(new Error('whatever'), { code: 4001 });
    expect(normalizeProviderError(err, 'fallback', 'evm-injected')).toMatchObject({
      code: 'user_rejected',
    });
  });

  it('preserves WalletAdapterError instances', () => {
    const original = normalizeProviderError(
      Object.assign(new Error('x'), { code: 4001 }),
      'fallback'
    );
    expect(normalizeProviderError(original, 'other')).toBe(original);
  });
});
