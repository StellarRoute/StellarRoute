import { describe, expect, it, vi } from 'vitest';
import { executeEvmPreparedPayload } from './evm-execution';

describe('executeEvmPreparedPayload receipt handling', () => {
  const payload = {
    type: 'evm_transaction' as const,
    chain_id: 'eip155:11155111',
    to: '0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
    data: '0x',
    value: '0',
  };

  it('returns pending without throwing when receipt is not confirmed', async () => {
    const result = await executeEvmPreparedPayload({
      payload,
      evmAdapterId: 'evm:injected',
      deps: {
        readChainIdHex: vi.fn().mockResolvedValue('0xaa36a7'),
        switchNetwork: vi.fn(),
        sendTransaction: vi.fn().mockResolvedValue({
          kind: 'evm_transaction' as const,
          hash: '0xabc',
        }),
        waitForReceipt: vi.fn().mockResolvedValue('pending'),
      },
    });
    expect(result).toEqual({ status: 'pending', txHash: '0xabc' });
  });

  it('throws on reverted receipt', async () => {
    await expect(
      executeEvmPreparedPayload({
        payload,
        evmAdapterId: 'evm:injected',
        deps: {
          readChainIdHex: vi.fn().mockResolvedValue('0xaa36a7'),
          switchNetwork: vi.fn(),
          sendTransaction: vi.fn().mockResolvedValue({
            kind: 'evm_transaction' as const,
            hash: '0xabc',
          }),
          waitForReceipt: vi.fn().mockResolvedValue('reverted'),
        },
      }),
    ).rejects.toThrow(/reverted/i);
  });
});
