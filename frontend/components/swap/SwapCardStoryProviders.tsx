'use client';

import { ReactNode, useEffect } from 'react';
import { ThemeProvider } from 'next-themes';
import { Toaster } from 'sonner';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { StoryWalletProvider } from '@/components/providers/wallet-provider';
import { TradingPairProvider } from '@/contexts/TradingPairContext';
import { STORAGE_KEY } from '@/hooks/useTradeFormStorage';
import type { SwapCardStoryFixture } from './swapCardStory';
import { getSwapCardStoryPresentation } from './swapCardStory';

const MOCK_ACCOUNT_BALANCE = {
  balances: [{ balance: '50.0000000', asset_type: 'native' as const }],
};

const MOCK_QUOTE_SUCCESS = {
  total: '1.0500',
  price_impact: '0.12',
  path: [],
  price: '0.1050',
  amount: '10',
};

function seedTradeForm(fixture: SwapCardStoryFixture) {
  const { seedFromAmount } = getSwapCardStoryPresentation(fixture);
  if (!seedFromAmount) {
    localStorage.removeItem(STORAGE_KEY);
    return;
  }

  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      amount: seedFromAmount,
      slippage: 0.5,
      deadline: 30,
      fromToken: 'native',
      toToken: 'USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W',
      side: 'sell',
      savedAt: Date.now(),
    }),
  );
}

function installStoryFetch(fixture: SwapCardStoryFixture) {
  const presentation = getSwapCardStoryPresentation(fixture);
  const originalFetch = globalThis.fetch.bind(globalThis);

  globalThis.fetch = ((input: RequestInfo | URL) => {
    const url = typeof input === 'string' ? input : input.toString();

    if (url.includes('/accounts/')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(MOCK_ACCOUNT_BALANCE),
      } as Response);
    }

    if (presentation.quoteLoading) {
      return new Promise(() => {});
    }

    if (presentation.quoteError) {
      return Promise.resolve({
        ok: false,
        status: 503,
        json: () =>
          Promise.resolve({
            error: { code: 'QUOTE_UNAVAILABLE', message: presentation.quoteError?.message },
          }),
      } as Response);
    }

    if (url.includes('/quote') || url.includes('/routes')) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(MOCK_QUOTE_SUCCESS),
      } as Response);
    }

    return originalFetch(input);
  }) as typeof fetch;

  return () => {
    globalThis.fetch = originalFetch;
  };
}

export function SwapCardStoryProviders({
  fixture,
  children,
}: {
  fixture: SwapCardStoryFixture;
  children: ReactNode;
}) {
  const { walletConnected } = getSwapCardStoryPresentation(fixture);

  useEffect(() => {
    localStorage.clear();
    seedTradeForm(fixture);
    return installStoryFetch(fixture);
  }, [fixture]);

  return (
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
      <SettingsProvider>
        <StoryWalletProvider connected={walletConnected}>
          <TradingPairProvider>
            <div className="dark min-h-screen bg-background text-foreground p-8">
              {children}
              <Toaster />
            </div>
          </TradingPairProvider>
        </StoryWalletProvider>
      </SettingsProvider>
    </ThemeProvider>
  );
}
