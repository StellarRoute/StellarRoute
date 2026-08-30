'use client';

import * as React from 'react';
import { usePathname } from 'next/navigation';
import { Suspense } from 'react';
import { RefreshCw, X } from 'lucide-react';

import { Header } from './header';
import { Footer } from './footer';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useSessionRecovery } from '@/components/providers/session-recovery-provider';
import { useFormStateRecovery } from '@/hooks/useFormStateRecovery';
import { WalletSyncBanner, NetworkStatusBanner } from '@/components/shared';
import { DebugOverlay } from '@/components/debug/DebugOverlay';
import { stellarRouteClient } from '@/lib/api/client';

interface AppShellProps {
  children: React.ReactNode;
}

/**
 * Application shell component that wraps all pages
 *
 * Features:
 * - Consistent layout structure across all pages
 * - Header and footer on all pages
 * - Skip link + landmark regions for keyboard a11y
 * - Persistent network status banner
 * - Responsive content area with appropriate max-width
 */
export function AppShell({ children }: AppShellProps) {
  const pathname = usePathname();
  const {
    isStale,
    isRecovering,
    beginRecovery,
    completeRecovery,
    dismissRecovery,
  } = useSessionRecovery();
  const { getSavedFormState } = useFormStateRecovery();

  const isFullWidth =
    pathname?.startsWith('/orderbook') || pathname?.startsWith('/analytics');
  const isLanding = pathname === '/';

  const handleRestore = async () => {
    beginRecovery();
    try {
      const recoveryData = getSavedFormState();
      if (
        recoveryData &&
        recoveryData.baseAsset &&
        recoveryData.quoteAsset &&
        recoveryData.amount
      ) {
        await stellarRouteClient.getQuote(
          recoveryData.baseAsset,
          recoveryData.quoteAsset,
          parseFloat(recoveryData.amount),
          'sell'
        );
      }
      completeRecovery();
    } catch (error) {
      console.error('Session recovery failed:', error);
      throw error;
    }
  };

  return (
    <div className="flex min-h-screen flex-col">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      <Header />
      <NetworkStatusBanner />
      <WalletSyncBanner />

      {(isStale || isRecovering) && (
        <div
          data-testid="session-recovery-banner"
          className="w-full border-b border-primary/30 bg-primary/10 px-4 py-3 text-foreground supports-[backdrop-filter]:backdrop-blur-md animate-in fade-in slide-in-from-top duration-300"
          role="alert"
          aria-live="polite"
        >
          <div className="container mx-auto flex flex-col sm:flex-row items-center justify-between gap-3 max-w-7xl">
            <div className="flex items-center gap-3">
              <span className="h-2 w-2 rounded-full bg-primary animate-pulse" />
              <p className="text-sm font-medium">
                Your session is stale. Would you like to restore your last trade form state and refresh the quote?
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleRestore}
                disabled={isRecovering}
                className="h-8 text-xs bg-primary/20 border-primary/40 hover:bg-primary/30 font-medium transition-all gap-1.5"
              >
                {isRecovering ? (
                  <RefreshCw className="h-3 w-3 animate-spin" />
                ) : (
                  <RefreshCw className="h-3 w-3" />
                )}
                {isRecovering ? 'Restoring...' : 'Restore'}
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={dismissRecovery}
                disabled={isRecovering}
                className="h-8 w-8 text-muted-foreground hover:text-foreground"
                aria-label="Dismiss banner"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      )}

      <main
        id="main-content"
        tabIndex={-1}
        className={cn(
          'flex-1 outline-none',
          isLanding
            ? 'w-full'
            : isFullWidth
              ? 'w-full'
              : 'container mx-auto w-full max-w-7xl px-3 py-6 sm:px-6 sm:py-8 lg:px-8'
        )}
      >
        {children}
      </main>

      <Footer />

      <Suspense fallback={null}>
        <DebugOverlay />
      </Suspense>
    </div>
  );
}
