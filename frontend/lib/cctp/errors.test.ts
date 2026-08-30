import { describe, expect, it } from 'vitest';
import { HorizonSubmitError } from '@/lib/wallet/submit';
import { isStaleSequenceError, mapCctpError } from './errors';

describe('mapCctpError sequence stale', () => {
  it('maps Horizon tx_bad_seq to re-prepare guidance', () => {
    const err = new HorizonSubmitError('Transaction failed: tx_bad_seq', {
      code: 'tx_bad_seq',
      transactionCode: 'tx_bad_seq',
      status: 400,
    });
    const mapped = mapCctpError(err);
    expect(mapped.kind).toBe('sequence_stale');
    expect(mapped.title).toBe('Account sequence out of date');
    expect(mapped.action).toBe('Re-prepare transaction');
    expect(mapped.message).toContain('Re-prepare the burn');
  });

  it('detects classic bad_sequence status on generic errors', () => {
    const err = new Error('Account sequence mismatch') as Error & {
      status?: string;
    };
    err.status = 'bad_sequence';
    expect(isStaleSequenceError(err)).toBe(true);
    const mapped = mapCctpError(err);
    expect(mapped.kind).toBe('sequence_stale');
    expect(mapped.action).toBe('Re-prepare transaction');
  });
});
