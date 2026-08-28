import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import { hydrateRoot } from 'react-dom/client';
import { act } from 'react';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';
import { SwapPageClient } from './SwapPageClient';

vi.mock('next/dynamic', () => ({
  default: () => {
    const Mock = () => <div data-testid="swap-card-mock">Swap card</div>;
    return Mock;
  },
}));

vi.mock('@/components/swap/OnboardingChecklist', () => ({
  OnboardingChecklist: () => null,
}));

vi.mock('@/components/swap/cross-chain/CrossChainSwapDeck', () => ({
  CrossChainSwapDeck: () => (
    <div data-testid="cross-chain-swap-deck">Cross-chain deck</div>
  ),
}));

vi.mock('@/src/components/RoutesBetaGate', () => ({
  RoutesBetaGate: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

function renderSwapPageTree() {
  return (
    <SettingsProvider>
      <WalletProvider>
        <SwapPageClient />
      </WalletProvider>
    </SettingsProvider>
  );
}

import { invalidateFlagCache } from '@/hooks/useFeatureFlag';

describe('SwapPageClient hydration with swap_ui_v2', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invalidateFlagCache();
    delete process.env.NEXT_PUBLIC_FLAGS_URL;
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = 'true';
    process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA = 'false';
  });

  it('hydrates without recoverable errors when env enables swap_ui_v2', () => {
    const container = document.createElement('div');
    container.innerHTML = renderToString(renderSwapPageTree());

    const recoverableErrors: string[] = [];
    const consoleErrors: string[] = [];
    const originalError = console.error;
    console.error = (...args: unknown[]) => {
      const message = String(args[0] ?? '');
      if (message.includes('Hydration')) consoleErrors.push(message);
      originalError(...args);
    };

    act(() => {
      hydrateRoot(container, renderSwapPageTree(), {
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
    expect(container.textContent).toContain('Cross-chain deck');
  });

  it('hydrates legacy surface when swap_ui_v2 env is off', () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = 'false';

    const container = document.createElement('div');
    container.innerHTML = renderToString(renderSwapPageTree());

    const recoverableErrors: string[] = [];
    act(() => {
      hydrateRoot(container, renderSwapPageTree(), {
        onRecoverableError: (error) => {
          recoverableErrors.push(
            error instanceof Error ? error.message : String(error),
          );
        },
      });
    });

    expect(recoverableErrors).toEqual([]);
    expect(container.textContent).toContain('Swap card');
    expect(container.textContent).not.toContain('Cross-chain deck');
  });
});
