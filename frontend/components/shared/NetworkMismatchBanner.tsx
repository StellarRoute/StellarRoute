'use client';

import { AlertTriangle, ExternalLink, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useWallet } from '@/components/providers/wallet-provider';
import { cn } from '@/lib/utils';

const WALLET_DOCS: Record<string, string> = {
  freighter: 'https://docs.freighter.app/docs/guide/gettingStarted',
  xbull: 'https://xbull.app/docs',
};

interface NetworkMismatchBannerProps {
  className?: string;
}

/**
 * Banner shown when connected wallet network differs from app target network
 * Blocks swap until resolved or user disconnects
 */
export function NetworkMismatchBanner({ className }: NetworkMismatchBannerProps) {
  const { networkMismatch, network, walletNetwork, walletId, disconnect } = useWallet();

  if (!networkMismatch) return null;

  const walletDocsUrl = walletId ? WALLET_DOCS[walletId] : null;

  return (
    <Alert
      variant="destructive"
      className={cn(
        'relative border-amber-500/50 bg-amber-500/10 text-amber-900 dark:text-amber-100',
        className
      )}
      role="alert"
      aria-live="assertive"
    >
      <AlertTriangle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
      <AlertDescription className="flex flex-col gap-3 pr-8">
        <div className="text-sm font-medium">
          Network mismatch detected
        </div>
        <div className="text-xs text-amber-800 dark:text-amber-200">
          Your wallet is connected to <strong>{walletNetwork}</strong>, but this app is set to{' '}
          <strong>{network}</strong>. Please switch your wallet network to continue.
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {walletDocsUrl && (
            <Button
              variant="outline"
              size="sm"
              asChild
              className="h-8 text-xs border-amber-600/30 hover:bg-amber-500/20"
            >
              <a
                href={walletDocsUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5"
              >
                How to switch network
                <ExternalLink className="h-3 w-3" />
              </a>
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={disconnect}
            className="h-8 text-xs border-amber-600/30 hover:bg-amber-500/20"
          >
            Disconnect wallet
          </Button>
        </div>
      </AlertDescription>
      <Button
        variant="ghost"
        size="icon"
        onClick={disconnect}
        className="absolute top-2 right-2 h-6 w-6 rounded-md hover:bg-amber-500/20"
        aria-label="Dismiss and disconnect"
      >
        <X className="h-4 w-4" />
      </Button>
    </Alert>
  );
}
