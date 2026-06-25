import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useWalletBalance } from './useWalletBalance';

describe('useWalletBalance', () => {
  beforeEach(() => {
    vi.spyOn(global, 'fetch').mockImplementation(() =>
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ balances: [] }),
      } as Response),
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null balances when disconnected', () => {
    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: 'native',
        isConnected: false,
        network: 'testnet',
      })
    );

    expect(result.current.balance).toBeNull();
    expect(result.current.spendableBalance).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('returns null balances when address is missing', () => {
    const { result } = renderHook(() =>
      useWalletBalance({
        address: null,
        asset: 'native',
        isConnected: true,
        network: 'testnet',
      })
    );

    expect(result.current.balance).toBeNull();
    expect(result.current.spendableBalance).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it('fetches native balance from Horizon testnet', async () => {
    global.fetch = mockHorizonAccount([
      { balance: '42.5000000', asset_type: 'native' },
    ]) as typeof fetch;

    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: 'native',
        isConnected: true,
        network: 'testnet',
      })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(global.fetch).toHaveBeenCalledWith(
      `https://horizon-testnet.stellar.org/accounts/${encodeURIComponent(TEST_ADDRESS)}`,
      expect.objectContaining({ signal: expect.any(AbortSignal) })
    );
    expect(result.current.balance).toBe('42.5000000');
    expect(result.current.spendableBalance).toBe(
      (42.5 - XLM_FEE_RESERVE).toFixed(7)
    );
    expect(result.current.error).toBeNull();
  });

  it('fetches token balance by code and issuer', async () => {
    const issuer = 'GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH';
    global.fetch = mockHorizonAccount([
      { balance: '50.0000000', asset_type: 'native' },
      {
        balance: '250.1234567',
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: issuer,
      },
    ]) as typeof fetch;

    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: `USDC:${issuer}`,
        isConnected: true,
        network: 'mainnet',
      })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(global.fetch).toHaveBeenCalledWith(
      `https://horizon.stellar.org/accounts/${encodeURIComponent(TEST_ADDRESS)}`,
      expect.objectContaining({ signal: expect.any(AbortSignal) })
    );
    expect(result.current.balance).toBe('250.1234567');
    expect(result.current.spendableBalance).toBe('250.1234567');
  });

  it('returns zero when asset is not held', async () => {
    global.fetch = mockHorizonAccount([
      { balance: '10.0000000', asset_type: 'native' },
    ]) as typeof fetch;

    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: 'USDC:GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH',
        isConnected: true,
        network: 'testnet',
      })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.balance).toBe('0');
    expect(result.current.spendableBalance).toBe('0');
  });

  it('sets error when Horizon responds with failure', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({
        ok: false,
        json: () => Promise.reject(new Error('not found')),
      })
    ) as typeof fetch;

    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: 'native',
        isConnected: true,
        network: 'testnet',
      })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.balance).toBeNull();
    expect(result.current.spendableBalance).toBeNull();
    expect(result.current.error?.message).toBe('Unable to load wallet balance.');
  });

  it('aborts in-flight fetch when dependencies change', async () => {
    let resolveFirst: (value: unknown) => void;
    const firstPromise = new Promise((resolve) => {
      resolveFirst = resolve;
    });

    global.fetch = vi.fn(() => firstPromise) as typeof fetch;

    const { rerender, unmount } = renderHook(
      (props: { asset: string }) =>
        useWalletBalance({
          address: TEST_ADDRESS,
          asset: props.asset,
          isConnected: true,
          network: 'testnet',
        }),
      { initialProps: { asset: 'native' } }
    );

    rerender({ asset: 'USDC:GABC' });

    await act(async () => {
      resolveFirst!({
        ok: true,
        json: () =>
          Promise.resolve({
            balances: [{ balance: '99.0000000', asset_type: 'native' }],
          }),
      });
    });

    unmount();
    expect(global.fetch).toHaveBeenCalled();
  });

  it('never returns hardcoded stub balance in production path', async () => {
    global.fetch = mockHorizonAccount([
      { balance: '7.2500000', asset_type: 'native' },
    ]) as typeof fetch;

    const { result } = renderHook(() =>
      useWalletBalance({
        address: TEST_ADDRESS,
        asset: 'native',
        isConnected: true,
        network: 'testnet',
      })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.balance).not.toBe('10000.0000000');
    expect(result.current.spendableBalance).not.toBe('10000.0000000');
  });
});
