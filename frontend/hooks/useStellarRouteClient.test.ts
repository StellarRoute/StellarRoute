import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { createStellarRouteClient } from '@/lib/api/client';
import { useStellarRouteClient } from '@/hooks/useStellarRouteClient';

const mockUseWallet = vi.fn();

vi.mock('@/components/providers/wallet-provider', () => ({
  useWallet: () => mockUseWallet(),
}));

vi.mock('@/lib/api/client', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api/client')>(
    '@/lib/api/client',
  );
  return {
    ...actual,
    createStellarRouteClient: vi.fn((baseUrl?: string) =>
      actual.createStellarRouteClient(baseUrl),
    ),
  };
});

describe('useStellarRouteClient', () => {
  afterEach(() => {
    vi.clearAllMocks();
    delete process.env.NEXT_PUBLIC_API_URL_TESTNET;
    delete process.env.NEXT_PUBLIC_API_URL_MAINNET;
    delete process.env.NEXT_PUBLIC_API_URL;
  });

  it('creates a client for the selected testnet API base', () => {
    process.env.NEXT_PUBLIC_API_URL_TESTNET = 'https://api-test.example.com/api/v1';
    mockUseWallet.mockReturnValue({ network: 'testnet' });

    renderHook(() => useStellarRouteClient());

    expect(createStellarRouteClient).toHaveBeenCalledWith(
      'https://api-test.example.com',
    );
  });

  it('recreates the client when the selected network changes', () => {
    process.env.NEXT_PUBLIC_API_URL_TESTNET = 'https://api-test.example.com/api/v1';
    process.env.NEXT_PUBLIC_API_URL_MAINNET = 'https://api-main.example.com/api/v1';
    mockUseWallet.mockReturnValue({ network: 'testnet' });

    const { rerender } = renderHook(() => useStellarRouteClient());
    expect(createStellarRouteClient).toHaveBeenCalledTimes(1);

    mockUseWallet.mockReturnValue({ network: 'mainnet' });
    rerender();

    expect(createStellarRouteClient).toHaveBeenLastCalledWith(
      'https://api-main.example.com',
    );
  });
});
