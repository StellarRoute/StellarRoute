import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  CLASSIC_EXECUTION_MODE,
  NETWORK_MISMATCH_COPY,
  conflictStatusFromDetails,
  createApiSwapExecution,
  isUserRejectionError,
  pathStepsToRouteHops,
  preflightClassicOneHop,
  resolveSwapExecutionMode,
  userCopyForSwapExecutionError,
} from './api-execution';
import { StellarRouteApiError, type StellarRouteClient } from '@/lib/api/client';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';
import type { PathStep } from '@/types';

afterEach(() => {
  vi.restoreAllMocks();
});

const TESTNET_PASSPHRASE = 'Test SDF Network ; September 2015';

function preparedFixture(overrides: Record<string, unknown> = {}) {
  return {
    quote_id: 'q-1',
    xdr_envelope: 'opaque_unsigned_xdr',
    expected_output: '9.91',
    min_output: '9.80',
    expires_at: Date.now() + 60_000,
    execution_mode: CLASSIC_EXECUTION_MODE,
    network_passphrase: TESTNET_PASSPHRASE,
    ...overrides,
  };
}

const classicPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GDUK',
    },
    price: '0.99',
    source: 'sdex',
    fee_bps: 30,
  },
];

const tradeParams: TradeParams = {
  fromAsset: 'XLM',
  fromAmount: '10',
  toAsset: 'USDC',
  toAmount: '9.9',
  exchangeRate: '0.99',
  priceImpact: '0.1',
  minReceived: '9.8000 USDC',
  networkFee: '0.00001 XLM',
  routePath: classicPath,
  walletAddress: 'GABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOP',
};

describe('preflightClassicOneHop', () => {
  it('accepts a single classic SDEX hop', () => {
    expect(preflightClassicOneHop(classicPath)).toEqual({
      ok: true,
      reason: 'ok',
      message: null,
    });
  });

  it('rejects multi-hop and AMM/Soroban routes', () => {
    expect(preflightClassicOneHop([...classicPath, ...classicPath]).reason).toBe(
      'multi_hop',
    );
    expect(
      preflightClassicOneHop([{ ...classicPath[0], source: 'amm:pool' }]).reason,
    ).toBe('amm_or_soroban');
  });
});

describe('resolveSwapExecutionMode', () => {
  it('defaults to API prepare/submit when real_xdr is enabled', () => {
    expect(
      resolveSwapExecutionMode({
        realXdrEnabled: true,
        isProduction: true,
      }),
    ).toEqual({ mode: 'api_prepare_submit' });
  });

  it('fails closed in production when real_xdr is disabled', () => {
    const mode = resolveSwapExecutionMode({
      realXdrEnabled: false,
      isProduction: true,
    });
    expect(mode.mode).toBe('disabled');
    if (mode.mode === 'disabled') {
      expect(mode.message).toMatch(/disabled in production/i);
    }
  });

  it('fails closed while flags are loading (no alternate path)', () => {
    const mode = resolveSwapExecutionMode({
      realXdrEnabled: false,
      flagsLoading: true,
      isProduction: false,
    });
    expect(mode.mode).toBe('disabled');
    if (mode.mode === 'disabled') {
      expect(mode.message).toMatch(/still loading/i);
    }
  });

  it('fails closed in non-production when real_xdr is disabled (no client-XDR fallback)', () => {
    const mode = resolveSwapExecutionMode({
      realXdrEnabled: false,
      isProduction: false,
    });
    expect(mode.mode).toBe('disabled');
    if (mode.mode === 'disabled') {
      expect(mode.message).toMatch(/no client-built XDR fallback/i);
    }
  });
});

describe('createApiSwapExecution', () => {
  it('happy path: signs server XDR, submits, confirms on Horizon', async () => {
    const prepareSwap = vi.fn().mockResolvedValue(preparedFixture());
    const submitSwap = vi.fn().mockResolvedValue({
      quote_id: 'q-1',
      tx_hash: 'tx-hash-1',
      status: 'pending',
      ledger: 42,
    });
    const client = { prepareSwap, submitSwap } as unknown as StellarRouteClient;
    const signTransaction = vi.fn().mockResolvedValue('opaque_signed_xdr');

    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(JSON.stringify({ successful: true, hash: 'tx-hash-1' }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction,
      confirmPollIntervalMs: 1,
      confirmTimeoutMs: 1_000,
    });

    const unsigned = await deps.buildXdr(tradeParams);
    expect(unsigned).toBe('opaque_unsigned_xdr');
    expect(prepareSwap).toHaveBeenCalledTimes(1);
    expect(deps.getPreparedAmounts()?.expected_output).toBe('9.91');

    const signed = await deps.signTransaction(unsigned);
    expect(signed).toBe('opaque_signed_xdr');

    const submitted = await deps.submitTransaction(signed);
    expect(submitted.hash).toBe('tx-hash-1');
    expect(submitSwap).toHaveBeenCalledWith({
      quote_id: 'q-1',
      signed_xdr: 'opaque_signed_xdr',
    });
  });

  it('fails closed when prepare returns empty XDR', async () => {
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({ quote_id: 'q-empty', xdr_envelope: '   ', expected_output: '1' }),
      ),
      submitSwap: vi.fn(),
    } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction: async (xdr) => xdr,
      confirmOnHorizon: false,
    });

    await expect(deps.buildXdr(tradeParams)).rejects.toThrow(/empty/i);
  });

  it('rejects unsupported execution_mode before signing', async () => {
    const signTransaction = vi.fn();
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({
          quote_id: 'q-bad',
          expected_output: '1',
          execution_mode: 'soroban_router',
        }),
      ),
      submitSwap: vi.fn(),
    } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction,
      confirmOnHorizon: false,
    });

    await expect(deps.buildXdr(tradeParams)).rejects.toMatchObject({
      code: 'unsupported_execution_mode',
    });
    expect(signTransaction).not.toHaveBeenCalled();
  });

  it('fails closed before signing on network passphrase mismatch', async () => {
    const signTransaction = vi.fn();
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({
          network_passphrase: 'Public Global Stellar Network ; September 2015',
        }),
      ),
      submitSwap: vi.fn(),
    } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction,
      confirmOnHorizon: false,
    });

    await expect(deps.buildXdr(tradeParams)).rejects.toThrow(NETWORK_MISMATCH_COPY);
    expect(signTransaction).not.toHaveBeenCalled();
  });

  it('user rejection does not call submit', async () => {
    const prepareSwap = vi.fn().mockResolvedValue(
      preparedFixture({ quote_id: 'q-rej', expected_output: '9.9' }),
    );
    const submitSwap = vi.fn();
    const client = { prepareSwap, submitSwap } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction: async () => {
        throw new Error('User declined access');
      },
      confirmOnHorizon: false,
    });

    const unsigned = await deps.buildXdr(tradeParams);
    await expect(deps.signTransaction(unsigned)).rejects.toThrow(/declined/i);
    expect(submitSwap).not.toHaveBeenCalled();
    expect(isUserRejectionError(new Error('User declined access'))).toBe(true);
  });

  it('ambiguous submit retries reuse the exact same body and never re-prepare/sign', async () => {
    const prepareSwap = vi.fn().mockResolvedValue(
      preparedFixture({ quote_id: 'q-retry', expected_output: '9.9' }),
    );
    const submitSwap = vi
      .fn()
      .mockRejectedValueOnce(
        new StellarRouteApiError(503, 'dependency_unavailable', 'Horizon down'),
      )
      .mockResolvedValueOnce({
        quote_id: 'q-retry',
        tx_hash: 'tx-retry',
        status: 'pending',
      });
    const client = { prepareSwap, submitSwap } as unknown as StellarRouteClient;
    const signTransaction = vi.fn().mockResolvedValue('opaque_signed_xdr');

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction,
      ambiguousSubmitRetries: 2,
      confirmOnHorizon: false,
    });

    const unsigned = await deps.buildXdr(tradeParams);
    const signed = await deps.signTransaction(unsigned);
    await deps.submitTransaction(signed);

    expect(prepareSwap).toHaveBeenCalledTimes(1);
    expect(signTransaction).toHaveBeenCalledTimes(1);
    expect(submitSwap).toHaveBeenCalledTimes(2);
    expect(submitSwap.mock.calls[0]?.[0]).toEqual({
      quote_id: 'q-retry',
      signed_xdr: 'opaque_signed_xdr',
    });
    expect(submitSwap.mock.calls[1]?.[0]).toEqual(submitSwap.mock.calls[0]?.[0]);
  });

  it('surfaces pending_reconcile when dependency remains unavailable', async () => {
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({ quote_id: 'q-pend', expected_output: '9.9' }),
      ),
      submitSwap: vi
        .fn()
        .mockRejectedValue(
          new StellarRouteApiError(503, 'dependency_unavailable', 'Horizon down'),
        ),
    } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction: async () => 'opaque_signed_xdr',
      ambiguousSubmitRetries: 1,
      confirmOnHorizon: false,
    });

    await deps.buildXdr(tradeParams);
    await deps.signTransaction('opaque_unsigned_xdr');
    await expect(deps.submitTransaction('opaque_signed_xdr')).rejects.toMatchObject({
      code: 'dependency_unavailable',
      details: expect.objectContaining({ status: 'pending_reconcile' }),
    });
  });

  it('does not rewrite permanent submit failures as pending_reconcile', async () => {
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({ quote_id: 'q-trust', expected_output: '9.9' }),
      ),
      submitSwap: vi.fn().mockRejectedValue(
        new StellarRouteApiError(
          422,
          'not_executable',
          'op_no_trust',
          { status: 'broadcast_failed' },
        ),
      ),
    } as unknown as StellarRouteClient;

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction: async () => 'opaque_signed_xdr',
      ambiguousSubmitRetries: 3,
      confirmOnHorizon: false,
    });

    await deps.buildXdr(tradeParams);
    await deps.signTransaction('opaque_unsigned_xdr');
    await expect(deps.submitTransaction('opaque_signed_xdr')).rejects.toMatchObject({
      code: 'not_executable',
      status: 422,
      details: expect.not.objectContaining({ status: 'pending_reconcile' }),
    });
    expect(client.submitSwap).toHaveBeenCalledTimes(1);
  });

  it('confirmation timeout fails closed', async () => {
    const client = {
      prepareSwap: vi.fn().mockResolvedValue(
        preparedFixture({ quote_id: 'q-to', expected_output: '9.9' }),
      ),
      submitSwap: vi.fn().mockResolvedValue({
        quote_id: 'q-to',
        tx_hash: 'tx-missing',
        status: 'pending',
      }),
    } as unknown as StellarRouteClient;

    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{}', { status: 404 }),
    );

    const deps = createApiSwapExecution({
      client,
      sender: tradeParams.walletAddress,
      slippageBps: 50,
      network: 'testnet',
      signTransaction: async () => 'opaque_signed_xdr',
      confirmTimeoutMs: 20,
      confirmPollIntervalMs: 5,
    });

    await deps.buildXdr(tradeParams);
    await deps.signTransaction('opaque_unsigned_xdr');
    await expect(deps.submitTransaction('opaque_signed_xdr')).rejects.toThrow(
      /may still reconcile/i,
    );
  });
});

describe('conflict / copy helpers', () => {
  it('reads details.status for active prepare vs already submitted', () => {
    expect(
      conflictStatusFromDetails({ status: 'active_prepare_exists' }),
    ).toBe('active_prepare_exists');
    expect(
      userCopyForSwapExecutionError(
        new StellarRouteApiError(409, 'duplicate_quote', 'conflict', {
          status: 'active_prepare_exists',
        }),
      ),
    ).toMatch(/active prepare/i);
    expect(
      userCopyForSwapExecutionError(
        new StellarRouteApiError(409, 'duplicate_quote', 'conflict', {
          status: 'already_submitted',
        }),
      ),
    ).toMatch(/already submitted/i);
  });

  it('maps unsupported_route and quote_expired safely', () => {
    expect(
      userCopyForSwapExecutionError(
        new StellarRouteApiError(422, 'unsupported_route', 'multi-hop'),
      ),
    ).toMatch(/classic one-hop/i);
    expect(
      userCopyForSwapExecutionError(
        new StellarRouteApiError(422, 'quote_expired', 'expired'),
      ),
    ).toMatch(/expired/i);
  });

  it('maps path steps for prepare body', () => {
    expect(pathStepsToRouteHops(classicPath)[0]?.from_asset).toBe('native');
  });
});
