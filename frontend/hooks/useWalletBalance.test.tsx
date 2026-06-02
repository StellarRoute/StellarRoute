import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useWalletBalance } from './useWalletBalance';

describe('useWalletBalance', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('loads asset balances from Horizon', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            balances: [
              { asset_type: 'native', balance: '20.0000000' },
              {
                asset_type: 'credit_alphanum4',
                asset_code: 'USDC',
                asset_issuer: 'GA123',
                balance: '42.5000000',
              },
            ],
          }),
      }),
    );

    const { result } = renderHook(() =>
      useWalletBalance({
        address: 'GABC123',
        asset: 'USDC:GA123',
        isConnected: true,
        network: 'testnet',
      }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(fetch).toHaveBeenCalledWith(
      'https://horizon-testnet.stellar.org/accounts/GABC123',
      expect.any(Object),
    );
    expect(result.current.balance).toBe('42.5000000');
    expect(result.current.spendableBalance).toBe('42.5000000');
    expect(result.current.error).toBeNull();
  });

  it('subtracts the fee reserve from native balances', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            balances: [{ asset_type: 'native', balance: '12.5000000' }],
          }),
      }),
    );

    const { result } = renderHook(() =>
      useWalletBalance({
        address: 'GABC123',
        asset: 'native',
        isConnected: true,
        network: 'testnet',
      }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(fetch).toHaveBeenCalledWith(
      'https://horizon-testnet.stellar.org/accounts/GABC123',
      expect.any(Object),
    );
    expect(result.current.balance).toBe('12.5000000');
    expect(result.current.spendableBalance).toBe('11.5000000');
  });
});
