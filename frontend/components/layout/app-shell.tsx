'use client';

import * as React from 'react';
import { usePathname } from 'next/navigation';

import { Header } from './header';
import { Footer } from './footer';
import { cn } from '@/lib/utils';
import { SessionRecoveryModal } from '@/components/modals/SessionRecoveryModal';
import { useSessionRecoveryContext } from '@/components/providers/session-recovery-provider';

interface AppShellProps {
  children: React.ReactNode;
}

/**
 * Application shell component that wraps all pages
 *
 * Features:
 * - Consistent layout structure across all pages
 * - Header and footer on all pages
 * - Responsive content area with appropriate max-width
 * - Centered content for swap-type pages
 * - Full-width content for orderbook/analytics pages
 * - Consistent spacing and padding system
 * - Session recovery modal on wake/refresh
 */
export function AppShell({ children }: AppShellProps) {
  const pathname = usePathname();
  const {
    isStale,
    isRecovering,
    refreshType,
    beginRecovery,
    completeRecovery,
    dismissRecovery,
  } = useSessionRecoveryContext();

  // Determine if page should be full-width (orderbook, analytics) or centered (swap)
  const isFullWidth =
    pathname?.startsWith('/orderbook') || pathname?.startsWith('/analytics');

  const handleRestore = async () => {
    beginRecovery();
    try {
      // Simulate quote refresh - in real app, this would be API call
      await new Promise((resolve) => setTimeout(resolve, 1000));
      completeRecovery();
    } catch (error) {
      console.error('Session recovery failed:', error);
      throw error;
    }
  };

  return (
    <div className="flex min-h-screen flex-col">
      <Header />

      <main
        className={cn(
          'flex-1',
          isFullWidth
            ? 'w-full'
            : 'container mx-auto w-full max-w-7xl px-3 py-8 sm:px-6 lg:px-8'
        )}
      >
        {children}
      </main>

      <Footer />

      <SessionRecoveryModal
        isOpen={isStale}
        isRecovering={isRecovering}
        refreshType={refreshType}
        onRestore={handleRestore}
        onDismiss={dismissRecovery}
      />
    </div>
  );
}
