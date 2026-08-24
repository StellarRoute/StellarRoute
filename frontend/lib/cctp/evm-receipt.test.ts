import { describe, expect, it, vi } from 'vitest';
import { pollReceiptViaProvider } from './evm-receipt';
import type { Eip1193Provider } from '@/lib/wallet/adapters/evm/provider';

describe('pollReceiptViaProvider', () => {
  it('returns success when receipt status is 0x1', async () => {
    const provider = {
      request: vi.fn().mockResolvedValue({ status: '0x1' }),
    } as unknown as Eip1193Provider;
    const status = await pollReceiptViaProvider(provider, '0xabc', {
      timeoutMs: 100,
      pollMs: 10,
    });
    expect(status).toBe('success');
    expect(provider.request).toHaveBeenCalledWith(
      expect.objectContaining({ method: 'eth_getTransactionReceipt' }),
    );
  });

  it('returns pending when no receipt before timeout', async () => {
    const provider = {
      request: vi.fn().mockResolvedValue(null),
    } as unknown as Eip1193Provider;
    const status = await pollReceiptViaProvider(provider, '0xabc', {
      timeoutMs: 30,
      pollMs: 10,
    });
    expect(status).toBe('pending');
  });
});
