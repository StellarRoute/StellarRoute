import { renderHook, act } from '@testing-library/react';
import { StrictMode, createElement, type ReactNode } from 'react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useCctpSaga } from './useCctpSaga';
import { fingerprintPreparedPayload } from '@/lib/cctp/payload-fingerprint';
import { buildCctpSessionRecord } from '@/lib/cctp/session-vault';
import { buildWalletRoleBindings } from '@/lib/cctp/wallet-role-binding';

const prepareBurn = vi.fn();
const submitBurn = vi.fn();
const prepareMint = vi.fn();
const submitMint = vi.fn();
const getTransfer = vi.fn();
const executePreparedPayload = vi.fn();
const startPoll = vi.fn(() => ({ stop: vi.fn() }));

vi.mock('@/lib/cctp/client', () => ({
  getCctpApiClient: () => ({
    quote: vi.fn().mockResolvedValue({
      transfer_id: 't1',
      access_token: 'tok',
      corridor_id: 'c',
      provider: 'circle-cctp',
      direction: 'evm_to_stellar',
      source_amount: '10',
      destination_amount: '9.9',
      fee_quote: {},
      expires_at: 9999999999,
      finality: 'standard',
    }),
    prepareBurn,
    submitBurn,
    prepareMint,
    submitMint,
    getTransfer,
  }),
}));

vi.mock('@/lib/cctp/wallet-execution', () => ({
  executePreparedPayload: (...args: unknown[]) => executePreparedPayload(...args),
  reconcileEvmTransactionHash: vi.fn(),
}));

vi.mock('@/lib/cctp/status-poll', () => ({
  startCctpStatusPoll: (...args: unknown[]) => startPoll(...args),
}));

const evmApprovalPayload = {
  type: 'evm_transaction' as const,
  chain_id: 'eip155:11155111',
  to: '0x1c7d4b196cb0c7b01d743fbc6116a902379c7238',
  data: '0xapprove',
  value: '0',
};

const evmBurnPayload = {
  ...evmApprovalPayload,
  data: '0xburn',
};

const stellarApprovalPayload = {
  type: 'stellar_xdr' as const,
  network_passphrase: 'Test SDF Network ; September 2015',
  xdr_envelope: 'AAAAapproval',
};

const stellarBurnPayload = {
  ...stellarApprovalPayload,
  xdr_envelope: 'AAAAburn',
};

const EVM_SOURCE = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0';
const STELLAR_G = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';

const defaultBindings = buildWalletRoleBindings({
  direction: 'evm_to_stellar',
  sourceChainId: 'eip155:11155111',
  destChainId: 'stellar:testnet',
  sender: EVM_SOURCE,
  recipient: STELLAR_G,
  mintSubmitter: STELLAR_G,
})!;

const baseInput = {
  sourceChainId: 'ethereum-sepolia' as const,
  destChainId: 'stellar' as const,
  amount: '10',
  recipient: STELLAR_G,
  wallets: {
    recipient: STELLAR_G,
    sourceEvmAdapterId: 'evm:test',
    sourceAddress: EVM_SOURCE,
    mintSubmitter: STELLAR_G,
    mintSubmitterStellarAdapterId: 'freighter',
  },
  bridgeReady: true,
  quoteInputsKey: 'k1',
  sender: EVM_SOURCE,
  mintSubmitter: STELLAR_G,
};

function seedSession(overrides?: {
  burnPrepareStep?: 'approval_ready' | 'burn_ready' | 'reprepare_required';
  fingerprint?: string;
  walletBindings?: ReturnType<typeof buildWalletRoleBindings>;
  version?: 1 | 2;
}) {
  const fp = overrides?.fingerprint ?? 'vault-fp';
  const record = buildCctpSessionRecord({
    transferId: 't1',
    accessToken: 'tok',
    idempotencyKey: 'idem-1',
    recovery: {
      corridorId: 'c',
      direction: 'evm_to_stellar',
      sourceChainId: 'ethereum-sepolia',
      destChainId: 'stellar',
      amount: '10',
      recipient: STELLAR_G,
      burnPrepareStep: overrides?.burnPrepareStep ?? 'approval_ready',
      lastPreparedFingerprint: fp,
      walletBindings: overrides?.walletBindings ?? defaultBindings,
    },
  });
  if (overrides?.version === 1) {
    record.version = 1;
    if (overrides.walletBindings === undefined) {
      delete record.recovery.walletBindings;
    }
  }
  sessionStorage.setItem('stellarroute:cctp:v1', JSON.stringify(record));
  return record;
}

describe('useCctpSaga server-driven burn staging', () => {
  beforeEach(() => {
    sessionStorage.clear();
    prepareBurn.mockReset();
    submitBurn.mockReset();
    getTransfer.mockReset();
    executePreparedPayload.mockReset();
    startPoll.mockClear();
    executePreparedPayload.mockResolvedValue({ txHash: '0xhash', submissionReady: true });
    submitBurn.mockResolvedValue({ status: 'burn_submitted' });
    getTransfer.mockResolvedValue({
      transfer_id: 't1',
      corridor_id: 'c',
      provider: 'circle-cctp',
      direction: 'evm_to_stellar',
      status: 'burn_prepared',
      retryable: false,
    });
  });

  it('EVM: prepare → approve (1 wallet) → prepare → burn (1 wallet)', async () => {
    const input = {
      sourceChainId: 'ethereum-sepolia' as const,
      destChainId: 'stellar' as const,
      amount: '10',
      recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
      wallets: {
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        sourceEvmAdapterId: 'evm:test',
        sourceAddress: EVM_SOURCE,
        mintSubmitter: STELLAR_G,
        mintSubmitterStellarAdapterId: 'freighter',
      },
      bridgeReady: true,
      quoteInputsKey: 'k1',
      sender: EVM_SOURCE,
      mintSubmitter: STELLAR_G,
    };
    prepareBurn
      .mockResolvedValueOnce({
        approval_required: true,
        payload: evmApprovalPayload,
        expires_at: 9999999999,
      })
      .mockResolvedValueOnce({
        approval_required: false,
        payload: evmBurnPayload,
        expires_at: 9999999999,
      });

    const { result } = renderHook(() => useCctpSaga(input));
    await act(async () => {
      await result.current.requestQuote();
    });
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    expect(result.current.burnPrepareStep).toBe('approval_ready');

    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(1);
    expect(submitBurn).toHaveBeenCalledTimes(1);
    // signApprovalStep re-prepares the burn payload after approval submit.
    expect(result.current.burnPrepareStep).toBe('burn_ready');
    const burnFingerprint = fingerprintPreparedPayload(evmBurnPayload);
    expect(result.current.getLastPreparedFingerprint()).toBe(burnFingerprint);

    await act(async () => {
      await result.current.signBurnStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(2);
    expect(prepareBurn).toHaveBeenCalledTimes(2);
  });

  it('Stellar: server approval_required drives Stellar approval then burn', async () => {
    const input = {
      sourceChainId: 'stellar' as const,
      destChainId: 'ethereum-sepolia' as const,
      amount: '10',
      recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      wallets: {
        recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
        sourceStellarAdapterId: 'freighter',
        sourceAddress: STELLAR_G,
        evmDestinationAdapterId: 'evm:dest',
      },
      bridgeReady: true,
      quoteInputsKey: 'k2',
      sender: STELLAR_G,
    };
    prepareBurn
      .mockResolvedValueOnce({
        approval_required: true,
        payload: stellarApprovalPayload,
        expires_at: 9999999999,
      })
      .mockResolvedValueOnce({
        approval_required: false,
        payload: stellarBurnPayload,
        expires_at: 9999999999,
      });

    const { result } = renderHook(() => useCctpSaga(input));
    await act(async () => {
      await result.current.requestQuote();
    });
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(1);
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    await act(async () => {
      await result.current.signBurnStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(2);
    const fp1 = fingerprintPreparedPayload(stellarApprovalPayload);
    const fp2 = fingerprintPreparedPayload(stellarBurnPayload);
    expect(fp1).not.toBe(fp2);
  });

  it('Stellar burn tx_bad_seq returns to re-prepare without failed stage', async () => {
    const input = {
      sourceChainId: 'stellar' as const,
      destChainId: 'ethereum-sepolia' as const,
      amount: '10',
      recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
      wallets: {
        recipient: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0',
        sourceStellarAdapterId: 'freighter',
        sourceAddress: STELLAR_G,
        evmDestinationAdapterId: 'evm:dest',
      },
      bridgeReady: true,
      quoteInputsKey: 'k-seq',
      sender: STELLAR_G,
    };
    prepareBurn.mockResolvedValue({
      approval_required: false,
      payload: stellarBurnPayload,
      expires_at: 9999999999,
    });
    const { HorizonSubmitError } = await import('@/lib/wallet/submit');
    executePreparedPayload.mockRejectedValue(
      new HorizonSubmitError('Transaction failed: tx_bad_seq', {
        code: 'tx_bad_seq',
        transactionCode: 'tx_bad_seq',
        status: 400,
      }),
    );

    const { result } = renderHook(() => useCctpSaga(input));
    await act(async () => {
      await result.current.requestQuote();
    });
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    expect(result.current.burnPrepareStep).toBe('burn_ready');

    await act(async () => {
      await result.current.signBurnStep();
    });

    expect(result.current.stage).toBe('quoted');
    expect(result.current.burnPrepareStep).toBe('reprepare_required');
    expect(result.current.primaryAction.label).toBe('Re-prepare transaction');
    expect(result.current.primaryAction.action).toBe('prepare');
    expect(result.current.error?.kind).toBe('sequence_stale');
    expect(submitBurn).not.toHaveBeenCalled();
  });

  it('does not submit burn when EVM receipt is pending', async () => {
    prepareBurn.mockResolvedValue({
      approval_required: false,
      payload: evmBurnPayload,
      expires_at: 9999999999,
    });
    executePreparedPayload.mockResolvedValue({
      txHash: '0xpending',
      submissionReady: false,
    });
    const { result } = renderHook(() =>
      useCctpSaga({
        sourceChainId: 'ethereum-sepolia',
        destChainId: 'stellar',
        amount: '10',
        recipient: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
        wallets: {
          recipient: STELLAR_G,
          sourceEvmAdapterId: 'evm:test',
          sourceAddress: EVM_SOURCE,
          mintSubmitter: STELLAR_G,
          mintSubmitterStellarAdapterId: 'freighter',
        },
        bridgeReady: true,
        quoteInputsKey: 'k3',
        sender: EVM_SOURCE,
        mintSubmitter: STELLAR_G,
      }),
    );
    await act(async () => {
      await result.current.requestQuote();
    });
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    await act(async () => {
      await result.current.signBurnStep();
    });
    expect(submitBurn).not.toHaveBeenCalled();
    expect(result.current.stage).toBe('pending_reconcile');
  });
});

describe('useCctpSaga reconcile stability', () => {
  beforeEach(() => {
    sessionStorage.clear();
    prepareBurn.mockReset();
    submitBurn.mockReset();
    getTransfer.mockReset();
    executePreparedPayload.mockReset();
    startPoll.mockClear();
    executePreparedPayload.mockResolvedValue({ txHash: '0xhash', submissionReady: true });
    submitBurn.mockResolvedValue({ status: 'burn_submitted' });
    getTransfer.mockResolvedValue({
      transfer_id: 't1',
      corridor_id: 'c',
      provider: 'circle-cctp',
      direction: 'evm_to_stellar',
      status: 'burn_prepared',
      retryable: false,
    });
  });

  it('auto-reconciles at most once per session revision (StrictMode safe)', async () => {
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: 'fp-vault' });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(StrictMode, null, children);

    const { rerender } = renderHook(() => useCctpSaga(baseInput), { wrapper });
    await act(async () => {
      await Promise.resolve();
    });
    rerender();
    await act(async () => {
      await Promise.resolve();
    });

    expect(getTransfer).toHaveBeenCalledTimes(1);
  });

  it('reload without in-memory payload requires re-prepare before Approve CTA', async () => {
    const fp = fingerprintPreparedPayload(evmApprovalPayload);
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: fp });

    const { result } = renderHook(() => useCctpSaga(baseInput));
    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.burnPrepareStep).toBe('reprepare_required');
    expect(result.current.primaryAction.label).toBe('Re-prepare transaction');
    expect(result.current.primaryAction.action).toBe('prepare');
  });

  it('reload after approval-ready: re-prepare then approve uses one wallet call', async () => {
    const fp = fingerprintPreparedPayload(evmApprovalPayload);
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: fp });
    prepareBurn.mockResolvedValue({
      approval_required: true,
      payload: evmApprovalPayload,
      expires_at: 9999999999,
    });

    const { result } = renderHook(() => useCctpSaga(baseInput));
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.burnPrepareStep).toBe('reprepare_required');

    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    expect(result.current.burnPrepareStep).toBe('approval_ready');
    expect(prepareBurn).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(1);
    // Initial prepare + post-approval re-prepare for the burn step.
    expect(prepareBurn).toHaveBeenCalledTimes(2);
  });

  it('retries auto-reconcile after vault form inputs are restored', async () => {
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: 'fp-vault' });

    const mismatchInput = {
      ...baseInput,
      sourceChainId: 'stellar' as const,
      destChainId: 'stellar' as const,
      quoteInputsKey: 'mismatch',
    };

    const { rerender } = renderHook(
      ({ input }: { input: typeof baseInput }) => useCctpSaga(input),
      { initialProps: { input: mismatchInput } },
    );
    await act(async () => {
      await Promise.resolve();
    });
    expect(getTransfer).not.toHaveBeenCalled();

    rerender({ input: baseInput });
    await act(async () => {
      await Promise.resolve();
    });
    expect(getTransfer).toHaveBeenCalledTimes(1);
  });

  it('manual resume always refreshes transfer status', async () => {
    seedSession();
    const { result } = renderHook(() => useCctpSaga(baseInput));
    await act(async () => {
      await Promise.resolve();
    });
    getTransfer.mockClear();

    await act(async () => {
      await result.current.resumeTransfer();
    });
    expect(getTransfer).toHaveBeenCalledTimes(1);
  });

  it('wrong reconnect blocks signing with zero wallet and submit calls', async () => {
    const fp = fingerprintPreparedPayload(evmApprovalPayload);
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: fp });
    prepareBurn.mockResolvedValue({
      approval_required: true,
      payload: evmApprovalPayload,
      expires_at: 9999999999,
    });

    const wrongWalletInput = {
      ...baseInput,
      wallets: {
        ...baseInput.wallets,
        sourceAddress: '0x1111111111111111111111111111111111111111',
      },
    };
    const { result } = renderHook(() => useCctpSaga(wrongWalletInput));
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.walletRoleMismatch?.code).toBe('source_burn_mismatch');

    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(prepareBurn).not.toHaveBeenCalled();
    expect(executePreparedPayload).not.toHaveBeenCalled();
    expect(submitBurn).not.toHaveBeenCalled();
  });

  it('correct reconnect resumes re-prepare then one wallet signature', async () => {
    const fp = fingerprintPreparedPayload(evmApprovalPayload);
    seedSession({ burnPrepareStep: 'approval_ready', fingerprint: fp });
    prepareBurn.mockResolvedValue({
      approval_required: true,
      payload: evmApprovalPayload,
      expires_at: 9999999999,
    });

    const { result } = renderHook(() => useCctpSaga(baseInput));
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.burnPrepareStep).toBe('reprepare_required');
    await act(async () => {
      await result.current.prepareSourceBurn();
    });
    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(executePreparedPayload).toHaveBeenCalledTimes(1);
    expect(submitBurn).toHaveBeenCalledTimes(1);
  });

  it('old vault schema without bindings never signs', async () => {
    seedSession({ version: 1, walletBindings: undefined });
    const { result } = renderHook(() => useCctpSaga(baseInput));
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.walletRoleMismatch?.code).toBe('bindings_missing');
    await act(async () => {
      await result.current.signApprovalStep();
    });
    expect(executePreparedPayload).not.toHaveBeenCalled();
  });
});

describe('useCctpSaga reverse mint trustline', () => {
  const trustlinePayload = {
    type: 'stellar_xdr' as const,
    network_passphrase: 'Test SDF Network ; September 2015',
    xdr_envelope: 'AAAAtrustline',
    source: STELLAR_G,
  };
  const mintPayload = {
    type: 'stellar_xdr' as const,
    network_passphrase: 'Test SDF Network ; September 2015',
    xdr_envelope: 'AAAAmint',
  };

  beforeEach(() => {
    sessionStorage.clear();
    prepareMint.mockReset();
    submitMint.mockReset();
    getTransfer.mockReset();
    executePreparedPayload.mockReset();
    startPoll.mockClear();
    executePreparedPayload.mockResolvedValue({
      txHash: 'stellar-hash',
      submissionReady: true,
    });
    submitMint.mockResolvedValue({ status: 'mint_submitted' });
    getTransfer.mockResolvedValue({
      transfer_id: 't1',
      corridor_id: 'c',
      provider: 'circle-cctp',
      direction: 'evm_to_stellar',
      status: 'attestation_ready',
      retryable: false,
    });
  });

  it('opens trustline then mints without submitting ChangeTrust as mint', async () => {
    prepareMint
      .mockResolvedValueOnce({
        transfer_id: 't1',
        status: 'attestation_ready',
        payload: trustlinePayload,
        expires_at: 9999999999,
        trustline_required: true,
      })
      .mockResolvedValueOnce({
        transfer_id: 't1',
        status: 'attestation_ready',
        payload: mintPayload,
        expires_at: 9999999999,
        trustline_required: false,
      });

    const { result } = renderHook(() =>
      useCctpSaga({
        ...baseInput,
        wallets: {
          recipient: STELLAR_G,
          sourceEvmAdapterId: 'evm:test',
          sourceAddress: EVM_SOURCE,
          mintSubmitter: STELLAR_G,
          mintSubmitterStellarAdapterId: 'freighter',
        },
      }),
    );
    await act(async () => {
      await result.current.requestQuote();
    });
    // Force attestation-ready path by calling mint step directly after quote.
    await act(async () => {
      await result.current.signPreparedMintStep();
    });

    expect(prepareMint).toHaveBeenCalledTimes(2);
    expect(executePreparedPayload).toHaveBeenCalledTimes(2);
    expect(executePreparedPayload.mock.calls[0][0].payload.xdr_envelope).toBe(
      'AAAAtrustline',
    );
    expect(executePreparedPayload.mock.calls[1][0].payload.xdr_envelope).toBe(
      'AAAAmint',
    );
    expect(submitMint).toHaveBeenCalledTimes(1);
    expect(submitMint.mock.calls[0][1]).toEqual({ tx_hash: 'stellar-hash' });
  });

  it('single mint prepare when trustline already present', async () => {
    prepareMint.mockResolvedValue({
      transfer_id: 't1',
      status: 'attestation_ready',
      payload: mintPayload,
      expires_at: 9999999999,
      trustline_required: false,
    });

    const { result } = renderHook(() =>
      useCctpSaga({
        ...baseInput,
        wallets: {
          recipient: STELLAR_G,
          sourceEvmAdapterId: 'evm:test',
          sourceAddress: EVM_SOURCE,
          mintSubmitter: STELLAR_G,
          mintSubmitterStellarAdapterId: 'freighter',
        },
      }),
    );
    await act(async () => {
      await result.current.requestQuote();
    });
    await act(async () => {
      await result.current.signPreparedMintStep();
    });

    expect(prepareMint).toHaveBeenCalledTimes(1);
    expect(executePreparedPayload).toHaveBeenCalledTimes(1);
    expect(submitMint).toHaveBeenCalledTimes(1);
  });
});
