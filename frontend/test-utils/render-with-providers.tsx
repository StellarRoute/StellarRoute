import React from 'react';
import { render } from '@testing-library/react';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { vi } from 'vitest';

// Mock Next.js navigation hooks globally for tests using this utility
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    pathname: '/',
    query: {},
  }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

export function renderWithProviders(
  ui: React.ReactElement,
  { walletInitialState = {} } = {}
) {
  return render(
    <WalletProvider initialState={walletInitialState}>
      {ui}
    </WalletProvider>
  );
}
