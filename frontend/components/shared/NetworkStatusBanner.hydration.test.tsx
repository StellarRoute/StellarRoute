import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import { hydrateRoot } from 'react-dom/client';
import { act } from 'react';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { NetworkStatusBanner } from './NetworkStatusBanner';

vi.mock('@/hooks/useApi', () => ({
  useHealth: () => ({
    data: { status: 'ok' },
    loading: false,
    error: null,
  }),
}));

function renderBannerTree() {
  return (
    <SettingsProvider>
      <WalletProvider>
        <NetworkStatusBanner />
      </WalletProvider>
    </SettingsProvider>
  );
}

describe('NetworkStatusBanner hydration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('hydrates without recoverable errors when API health resolves before mount', () => {
    const container = document.createElement('div');
    container.innerHTML = renderToString(renderBannerTree());

    const recoverableErrors: string[] = [];
    const consoleErrors: string[] = [];
    const originalError = console.error;
    console.error = (...args: unknown[]) => {
      const message = String(args[0] ?? '');
      if (message.includes('Hydration')) consoleErrors.push(message);
      originalError(...args);
    };

    act(() => {
      hydrateRoot(container, renderBannerTree(), {
        onRecoverableError: (error) => {
          recoverableErrors.push(
            error instanceof Error ? error.message : String(error),
          );
        },
      });
    });

    console.error = originalError;

    expect(recoverableErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
    expect(container.textContent).toMatch(/Checking API|API reachable/);
  });

  it('shows API reachable after mount when health is available', async () => {
    const { render } = await import('@testing-library/react');
    const view = render(renderBannerTree());

    await act(async () => {
      await Promise.resolve();
    });

    expect(view.getByText('API reachable')).toBeInTheDocument();
  });
});
