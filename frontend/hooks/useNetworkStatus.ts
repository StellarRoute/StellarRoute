'use client';

import { useEffect, useState } from 'react';

/**
 * Network status for the card deck (CARD-55).
 *
 * Listens to the browser `online` / `offline` window events and mirrors
 * `navigator.onLine`, so card action triggers can pause while the client is
 * disconnected from the Horizon network and RPC endpoints.
 *
 * Read-only: this hook never initiates network traffic itself.
 */
export function useNetworkStatus() {
  const [isOnline, setIsOnline] = useState<boolean>(() =>
    typeof navigator === 'undefined' ? true : navigator.onLine,
  );

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return { isOnline, isOffline: !isOnline };
}
