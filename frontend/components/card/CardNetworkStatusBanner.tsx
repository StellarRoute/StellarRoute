'use client';

import type { ReactNode } from 'react';

import { useNetworkStatus } from '@/hooks/useNetworkStatus';

export interface CardNetworkStatusBannerProps {
  /**
   * Card action triggers (for example top-up and freeze buttons).
   * Rendered inside a disabled fieldset while the client is offline, so the
   * triggers cannot start contract transactions without connectivity.
   * Read-only content (for example the cached card face) stays outside
   * this component and is never disabled.
   */
  children?: ReactNode;
}

/**
 * Offline indicator for the card deck (CARD-55).
 *
 * Shows an amber alert banner as soon as the client goes offline and hides
 * it automatically when connectivity returns. While offline, any action
 * triggers passed as children are disabled; nothing else on the page is
 * affected.
 */
export function CardNetworkStatusBanner({
  children,
}: CardNetworkStatusBannerProps) {
  const { isOnline } = useNetworkStatus();

  return (
    <div>
      {!isOnline && (
        <div
          role="alert"
          data-testid="card-network-status-banner"
          className="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200"
        >
          <p className="font-medium">You are offline</p>
          <p className="mt-1">
            Card actions are paused until your connection returns. Your cached
            card status is still available.
          </p>
        </div>
      )}
      {children ? (
        <fieldset
          disabled={!isOnline}
          data-testid="card-network-actions"
          className="m-0 min-w-0 border-0 p-0"
        >
          {children}
        </fieldset>
      ) : null}
    </div>
  );
}
