'use client';

import { ShieldAlert, RefreshCw, ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useWallet } from '@/components/providers/wallet-provider';
import { cn } from '@/lib/utils';
import { useCallback, useMemo } from 'react';

const WALLET_DOCS: Record<string, string> = {
  freighter: 'https://docs.freighter.app/docs/guide/gettingStarted',
  xbull: 'https://xbull.app/docs',
  albedo: 'https://albedo.link/',
  lobstr: 'https://lobstr.co/',
};

interface WalletCapabilitiesBannerProps {
  className?: string;
}

function getCapabilityLabel(capability: string): string {
  switch (capability) {
    case 'sign_transaction':
      return 'Sign transactions';
    case 'view_address':
      return 'View address';
    case 'view_network':
      return 'View network';
    case 'request_access':
      return 'Wallet access';
    default:
      return capability;
  }
}

function isNetworkRelatedDenial(reason?: string, resolution?: string): boolean {
  const haystack = `${reason ?? ''} ${resolution ?? ''}`.toLowerCase();
  return (
    haystack.includes('network mismatch') ||
    haystack.includes('switch wallet network') ||
    haystack.includes('expected testnet') ||
    haystack.includes('expected mainnet')
  );
}

export function WalletCapabilitiesBanner({ className }: WalletCapabilitiesBannerProps) {
  const { capabilities, walletId, refreshCapabilities, networkMismatch } = useWallet();

  const handleRefresh = useCallback(() => {
    void refreshCapabilities();
  }, [refreshCapabilities]);

  const denied = useMemo(() => {
    if (!capabilities) return [];
    return capabilities.statuses.filter((s) => {
      if (s.allowed) return false;
      // Avoid duplicating the dedicated network-mismatch banners
      if (networkMismatch && isNetworkRelatedDenial(s.reason, s.resolution)) {
        return false;
      }
      return true;
    });
  }, [capabilities, networkMismatch]);

  if (!capabilities || denied.length === 0) return null;

  const walletDocsUrl = walletId ? WALLET_DOCS[walletId] : null;

  return (
    <div
      role="alert"
      aria-live="polite"
      className={cn(
        'relative overflow-hidden rounded-2xl border border-destructive/30 bg-destructive/8 p-4 text-foreground',
        className
      )}
    >
      <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-destructive/50 to-transparent" />

      <div className="flex items-start gap-3">
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-destructive/15 text-destructive">
          <ShieldAlert className="h-4 w-4" aria-hidden />
        </span>

        <div className="min-w-0 flex-1 space-y-3">
          <div className="space-y-1">
            <p className="text-sm font-semibold tracking-tight">
              Wallet permissions required
            </p>
            <p className="text-xs text-muted-foreground">
              Approve the missing permissions in your wallet, then check again.
            </p>
          </div>

          <ul className="space-y-2">
            {denied.map((status) => (
              <li
                key={status.capability}
                className="rounded-lg border border-border/40 bg-background/35 px-3 py-2"
              >
                <p className="text-xs font-semibold">
                  {getCapabilityLabel(status.capability)}
                </p>
                {status.reason && (
                  <p className="mt-0.5 text-xs text-muted-foreground">{status.reason}</p>
                )}
                {status.resolution && (
                  <p className="mt-1 text-xs font-medium text-foreground/90">
                    {status.resolution}
                  </p>
                )}
              </li>
            ))}
          </ul>

          <div className="flex flex-wrap items-center gap-2">
            {walletDocsUrl && (
              <Button
                variant="outline"
                size="sm"
                asChild
                className="h-9 border-border/50 bg-background/50 text-xs"
              >
                <a
                  href={walletDocsUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1.5"
                >
                  Wallet docs
                  <ExternalLink className="h-3 w-3" />
                </a>
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              onClick={handleRefresh}
              className="h-9 border-border/50 bg-background/50 text-xs"
            >
              <RefreshCw className="mr-1.5 h-3 w-3" />
              Check again
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
