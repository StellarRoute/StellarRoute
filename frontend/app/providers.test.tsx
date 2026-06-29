import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { Providers } from './providers';

vi.mock('next-themes', () => ({
  ThemeProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/providers/session-recovery-provider', () => ({
  SessionRecoveryProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/providers/settings-provider', () => ({
  SettingsProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/providers/wallet-provider', () => ({
  WalletProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/contexts/TradingPairContext', () => ({
  TradingPairProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/providers/GlobalToastListener', () => ({
  GlobalToastListener: () => <div data-testid="global-toast-listener" />,
}));

describe('Providers', () => {
  it('mounts GlobalToastListener once at the app root', () => {
    render(
      <Providers>
        <div>content</div>
      </Providers>,
    );

    expect(screen.getByTestId('global-toast-listener')).toBeInTheDocument();
    expect(screen.getAllByTestId('global-toast-listener')).toHaveLength(1);
  });
});
