'use client';

import * as React from 'react';
import { AlertTriangle, RefreshCw, X } from 'lucide-react';
import { useWallet } from '@/components/providers/wallet-provider';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface WalletSyncBannerProps {
  className?: string;
}

/**
 * Global banner displayed when a wallet connection mismatch is detected
 * across multiple browser tabs.
 */
export function WalletSyncBanner({ className }: WalletSyncBannerProps) {
  const {
    syncMismatch,
    resyncWallet,
    dismissSyncMismatch,
    isTransactionPending,
  } = useWallet();

  const [storedAddress, setStoredAddress] = React.useState<string | null>(null);
  const [isSyncing, setIsSyncing] = React.useState(false);

  // Sync state from localStorage on mount or when banner is shown
  React.useEffect(() => {
    if (!syncMismatch || typeof window === 'undefined') return;

    const address = window.localStorage.getItem('stellarroute.wallet.address');
    setStoredAddress(address);
  }, [syncMismatch]);

  if (!syncMismatch) return null;

  const handleSync = async () => {
    setIsSyncing(true);
    try {
      await resyncWallet();
    } catch (error) {
      console.error('Failed to synchronize wallet across tabs:', error);
    } finally {
      setIsSyncing(false);
    }
  };

  const formatAddress = (addr: string) => {
    return `${addr.slice(0, 6)}...${addr.slice(-6)}`;
  };

  return (
    <div
      data-testid="wallet-sync-banner"
      className={cn(
        'w-full border-b border-amber-500/30 bg-amber-500/10 px-4 py-3 text-amber-200 supports-[backdrop-filter]:backdrop-blur-md animate-in fade-in slide-in-from-top duration-300',
        className
      )}
      role="alert"
      aria-live="polite"
    >
      <div className="container mx-auto flex flex-col sm:flex-row items-center justify-between gap-3 max-w-7xl">
        <div className="flex items-center gap-3">
          <AlertTriangle className="h-5 w-5 text-amber-500 flex-shrink-0 animate-pulse" />
          <p data-testid="wallet-sync-message" className="text-sm font-medium">
            {storedAddress ? (
              <>
                Wallet change detected in another tab. Click sync to switch to account{' '}
                <code className="rounded bg-amber-500/20 px-1 py-0.5 text-xs text-amber-300 font-mono">
                  {formatAddress(storedAddress)}
                </code>
                .
              </>
            ) : (
              'Wallet disconnected in another tab. Sync to disconnect here.'
            )}
          </p>
        </div>

        <div className="flex items-center gap-2 w-full sm:w-auto justify-end">
          <Button
            data-testid="wallet-sync-button"
            variant="outline"
            size="sm"
            onClick={handleSync}
            disabled={isSyncing || isTransactionPending}
            className="h-8 text-xs bg-amber-500/20 border-amber-500/40 text-amber-100 hover:bg-amber-500/30 font-medium transition-all gap-1.5"
          >
            {isSyncing ? (
              <RefreshCw className="h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="h-3 w-3" />
            )}
            {isSyncing ? 'Syncing...' : 'Sync Wallet'}
          </Button>

          <Button
            data-testid="wallet-dismiss-button"
            variant="ghost"
            size="icon"
            onClick={dismissSyncMismatch}
            disabled={isSyncing}
            className="h-8 w-8 text-amber-400 hover:text-amber-200 hover:bg-amber-500/20 rounded-md"
            aria-label="Dismiss banner"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
