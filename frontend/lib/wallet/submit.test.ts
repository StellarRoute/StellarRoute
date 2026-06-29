import { describe, it, expect, vi, afterEach } from 'vitest';
import { getHorizonUrl, getNetworkPassphrase, submitToHorizon } from './submit';

describe('getHorizonUrl', () => {
  it('returns testnet URL for testnet', () => {
    expect(getHorizonUrl('testnet')).toBe('https://horizon-testnet.stellar.org');
  });

  it('returns mainnet URL for mainnet', () => {
    expect(getHorizonUrl('mainnet')).toBe('https://horizon.stellar.org');
  });

  it('defaults to testnet for unknown network', () => {
    expect(getHorizonUrl('futurenet')).toBe('https://horizon-testnet.stellar.org');
  });

  it('defaults to testnet for null', () => {
    expect(getHorizonUrl(null)).toBe('https://horizon-testnet.stellar.org');
  });
});

describe('getNetworkPassphrase', () => {
  it('returns testnet passphrase', () => {
    expect(getNetworkPassphrase('testnet')).toBe('Test SDF Network ; September 2015');
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

    await expect(submitToHorizon('bad_xdr', 'testnet')).rejects.toThrow(
      'Transaction failed: tx_bad_auth'
    );
  });

  it('throws with HTTP status when no JSON body', async () => {
    global.fetch = vi.fn().mockResolvedValueOnce({
      ok: false,
      status: 503,
      json: async () => { throw new Error('not json'); },
    } as Response);

    await expect(submitToHorizon('xdr', 'testnet')).rejects.toThrow('HTTP 503');
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
