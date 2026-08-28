import { describe, expect, it, vi, beforeEach } from 'vitest';
import { executePreparedPayload, reconcileEvmTransactionHash } from './wallet-execution';

vi.mock('@/lib/wallet/adapters', () => ({
  signWithChainWallet: vi.fn(),
}));

vi.mock('@/lib/wallet/submit', () => ({
  submitToHorizon: vi.fn(),
  getHorizonUrl: vi.fn(() => 'https://horizon-testnet.stellar.org'),
}));

vi.mock('./evm-execution', () => ({
  executeEvmPreparedPayload: vi.fn(),
}));

vi.mock('./evm-receipt', () => ({
  pollEvmTransactionReceipt: vi.fn(),
}));

vi.mock('@stellar/stellar-base', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@stellar/stellar-base')>();
  return {
    ...actual,
    TransactionBuilder: {
      fromXDR: () => ({
        hash: () => Buffer.from('abc123', 'hex'),
      }),
    },
  };
});

import { signWithChainWallet } from '@/lib/wallet/adapters';
import { submitToHorizon } from '@/lib/wallet/submit';
import { executeEvmPreparedPayload } from './evm-execution';
import { pollEvmTransactionReceipt } from './evm-receipt';

const STELLAR_PASS = 'Test SDF Network ; September 2015';

describe('wallet-execution', () => {
  beforeEach(() => {
    vi.mocked(signWithChainWallet).mockReset();
    vi.mocked(submitToHorizon).mockReset();
    vi.mocked(executeEvmPreparedPayload).mockReset();
    vi.mocked(pollEvmTransactionReceipt).mockReset();
  });

  it('uses payload network_passphrase for Horizon recovery lookup', async () => {
    vi.mocked(signWithChainWallet).mockResolvedValue({
      kind: 'stellar_xdr',
      signedXdr: 'signed-envelope-xdr',
    });
    vi.mocked(submitToHorizon).mockRejectedValue(new Error('duplicate'));
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ hash: 'abc123' }),
    }) as unknown as typeof fetch;

    const result = await executePreparedPayload({
      payload: {
        type: 'stellar_xdr',
        network_passphrase: STELLAR_PASS,
        xdr_envelope: 'AAAAxdr',
      },
      stellarAdapterId: 'freighter',
      walletNetwork: 'testnet',
    });

    expect(result.submissionReady).toBe(true);
    expect(result.txHash).toBe('abc123');
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('horizon-testnet'),
    );
  });

  it('does not submit API when EVM receipt is pending', async () => {
    vi.mocked(executeEvmPreparedPayload).mockResolvedValue({
      status: 'pending',
      txHash: '0xdead',
    });
    const result = await executePreparedPayload({
      payload: {
        type: 'evm_transaction',
        chain_id: 'eip155:11155111',
        to: '0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
        data: '0x',
        value: '0',
      },
      evmAdapterId: 'evm:injected',
    });
    expect(result).toEqual({ txHash: '0xdead', submissionReady: false });
  });

  it('reconcileEvmTransactionHash returns submissionReady on success', async () => {
    vi.mocked(pollEvmTransactionReceipt).mockResolvedValue('success');
    const result = await reconcileEvmTransactionHash({ txHash: '0xabc' });
    expect(result).toEqual({ txHash: '0xabc', submissionReady: true });
  });
});
