import { describe, expect, it } from 'vitest';
import { StellarRouteApiError } from '@/lib/api/client';
import {
  SAFE_LIFECYCLE_STATUSES,
  sanitizeLifecycleStatus,
  toLifecycleError,
} from './lifecycle-error';

describe('lifecycle-error allowlist', () => {
  it('includes curated swap statuses', () => {
    expect(SAFE_LIFECYCLE_STATUSES).toEqual(
      expect.arrayContaining([
        'bad_sequence',
        'missing_network_passphrase',
        'submitting_without_hash',
        'network_mismatch',
      ]),
    );
  });

  it('sanitizes allowlisted statuses and drops arbitrary details', () => {
    expect(sanitizeLifecycleStatus('bad_sequence')).toBe('bad_sequence');
    expect(sanitizeLifecycleStatus('missing_network_passphrase')).toBe(
      'missing_network_passphrase',
    );
    expect(sanitizeLifecycleStatus('submitting_without_hash')).toBe(
      'submitting_without_hash',
    );
    expect(sanitizeLifecycleStatus('not_a_real_status')).toBeUndefined();

    const err = toLifecycleError(
      new StellarRouteApiError(409, 'duplicate_quote', 'conflict', {
        status: 'submitting_without_hash',
        secret: 'should-not-copy',
        raw: { nested: true },
      }),
    );
    expect(err).toEqual({
      message: 'conflict',
      code: 'duplicate_quote',
      status: 'submitting_without_hash',
    });
    expect(err).not.toHaveProperty('details');
    expect(JSON.stringify(err)).not.toContain('should-not-copy');
  });
});
