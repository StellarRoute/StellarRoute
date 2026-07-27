import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  HorizonSubmitError,
  getHorizonUrl,
  getNetworkPassphrase,
  submitToHorizon,
} from './submit';

describe('getHorizonUrl', () => {
  it('returns testnet URL for testnet', () => {
    expect(getHorizonUrl('testnet')).toBe(
      'https://horizon-testnet.stellar.org'
    );
  });

  it('returns mainnet URL for mainnet', () => {
    expect(getHorizonUrl('mainnet')).toBe('https://horizon.stellar.org');
  });

  it('defaults to testnet for unknown network', () => {
    expect(getHorizonUrl('futurenet')).toBe(
      'https://horizon-testnet.stellar.org'
    );
  });

  it('defaults to testnet for null', () => {
    expect(getHorizonUrl(null)).toBe('https://horizon-testnet.stellar.org');
  });
});

describe('getNetworkPassphrase', () => {
  it('returns testnet passphrase', () => {
    expect(getNetworkPassphrase('testnet')).toBe(
      'Test SDF Network ; September 2015'
    );
  });

  it('returns mainnet passphrase', () => {
    expect(getNetworkPassphrase('mainnet')).toBe(
      'Public Global Stellar Network ; September 2015'
    );
  });

  it('returns futurenet passphrase', () => {
    expect(getNetworkPassphrase('futurenet')).toBe(
      'Test SDF Future Network ; October 2022'
    );
  });
});

describe('submitToHorizon', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns hash on successful submission', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ hash: 'abc123', ledger: 42 }),
    } as Response);

    const result = await submitToHorizon('signed_xdr', 'testnet');
    expect(result.hash).toBe('abc123');
    expect(result.ledger).toBe(42);
  });

  it('throws with Horizon result_codes on failure', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 400,
      json: async () => ({
        extras: { result_codes: { transaction: 'tx_bad_auth' } },
      }),
    } as Response);

    await expect(submitToHorizon('bad_xdr', 'testnet')).rejects.toMatchObject({
      code: 'tx_bad_auth',
      transactionCode: 'tx_bad_auth',
    });
  });

  it('throws typed op_underfunded errors', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 400,
      json: async () => ({
        extras: {
          result_codes: {
            transaction: 'tx_failed',
            operations: ['op_underfunded'],
          },
        },
      }),
    } as Response);

    await expect(submitToHorizon('bad_xdr', 'testnet')).rejects.toMatchObject({
      code: 'op_underfunded',
      operationCodes: ['op_underfunded'],
    });
  });

  it('throws with HTTP status when no JSON body', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 503,
      json: async () => {
        throw new Error('not json');
      },
    } as Response);

    await expect(submitToHorizon('xdr', 'testnet')).rejects.toMatchObject({
      code: 'horizon_error',
      message: 'HTTP 503: Transaction submission failed',
    });
  });

  it('throws typed timeout errors', async () => {
    global.fetch = vi.fn(
      (_url, init) =>
        new Promise((_resolve, reject) => {
          const signal = (init as RequestInit).signal as AbortSignal;
          signal.addEventListener('abort', () =>
            reject(new DOMException('Aborted', 'AbortError'))
          );
        })
    ) as typeof fetch;

    await expect(
      submitToHorizon('xdr', 'testnet', { timeoutMs: 1 })
    ).rejects.toBeInstanceOf(HorizonSubmitError);
    await expect(
      submitToHorizon('xdr', 'testnet', { timeoutMs: 1 })
    ).rejects.toMatchObject({ code: 'timeout' });
  });

  it('posts signed XDR to correct Horizon endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce({
      ok: true,
      json: async () => ({ hash: 'xyz', ledger: 1 }),
    } as Response);
    global.fetch = fetchMock;

    await submitToHorizon('my_xdr', 'mainnet');

    expect(fetchMock).toHaveBeenCalledWith(
      'https://horizon.stellar.org/transactions',
      expect.objectContaining({ method: 'POST' })
    );
  });
});
