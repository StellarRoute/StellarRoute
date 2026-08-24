import { describe, expect, it } from 'vitest';
import { buildCctpTimelineFromTransfer } from './cctp-timeline';
import type { CctpTransferStatusResponse } from '@/lib/cctp/types';

function transfer(
  overrides: Partial<CctpTransferStatusResponse> = {},
): CctpTransferStatusResponse {
  return {
    transfer_id: 'cctp-test',
    corridor_id: 'circle-cctp:usdc:stellar-testnet:ethereum-sepolia',
    provider: 'circle-cctp',
    direction: 'stellar_to_evm',
    status: 'awaiting_attestation',
    retryable: true,
    support_reference_id: 'cctp-ref-1',
    source_tx_hash: 'sourcehash',
    ...overrides,
  };
}

describe('buildCctpTimelineFromTransfer', () => {
  it('marks burn complete and attest active while awaiting attestation', () => {
    const steps = buildCctpTimelineFromTransfer(transfer());
    expect(steps.find((s) => s.id === 'burn')?.status).toBe('complete');
    expect(steps.find((s) => s.id === 'attest')?.status).toBe('active');
    expect(steps.find((s) => s.id === 'mint')?.status).toBe('pending');
  });

  it('marks the full corridor complete when transfer completed', () => {
    const steps = buildCctpTimelineFromTransfer(
      transfer({
        status: 'completed',
        destination_tx_hash: 'desthash',
      }),
    );
    expect(steps.find((s) => s.id === 'burn')?.status).toBe('complete');
    expect(steps.find((s) => s.id === 'attest')?.status).toBe('complete');
    expect(steps.find((s) => s.id === 'mint')?.status).toBe('complete');
    expect(steps.find((s) => s.id === 'mint')?.href).toContain('sepolia.etherscan.io');
  });
});
