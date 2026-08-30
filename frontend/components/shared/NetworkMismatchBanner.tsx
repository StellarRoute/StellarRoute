'use client';

import { AlertTriangle, ArrowRight, ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useWallet } from '@/components/providers/wallet-provider';
import { isNetworkAllowed } from '@/lib/network-policy';
import { cn } from '@/lib/utils';

const WALLET_DOCS: Record<string, string> = {
  freighter: 'https://docs.freighter.app/docs/guide/gettingStarted',
  xbull: 'https://xbull.app/docs',
  albedo: 'https://albedo.link/',
  lobstr: 'https://lobstr.co/',
};

function formatNetworkLabel(value: string | null | undefined): string {
  if (!value) return 'Unknown';
  const key = value.toLowerCase();
  if (key === 'public' || key === 'mainnet' || key === 'pubnet') return 'Mainnet';
  if (key === 'testnet' || key === 'test') return 'Testnet';
  return value;
}

interface NetworkMismatchBannerProps {
  className?: string;
}

/**
 * In-card network mismatch panel.
 * Shell already shows NetworkStatusBanner; this stays compact and action-focused.
 * Critical mismatch cannot be dismissed away.
 */
export function NetworkMismatchBanner({ className }: NetworkMismatchBannerProps) {
  const { networkMismatch, network, walletNetwork, walletId, disconnect, setNetwork } =
    useWallet();

  if (!networkMismatch) return null;

  const walletDocsUrl = walletId ? WALLET_DOCS[walletId] : null;
  const canUseWalletNetwork =
    walletNetwork !== null && isNetworkAllowed(walletNetwork);

  return (
    <div
      role="alert"
      aria-live="assertive"
      className={cn(
        'relative overflow-hidden rounded-2xl border border-signal/35 bg-signal/8 p-4 text-foreground',
        className
      )}
    >
      <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-signal/60 to-transparent" />

      <div className="flex items-start gap-3">
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-signal/15 text-signal">
          <AlertTriangle className="h-4 w-4" aria-hidden />
        </span>

        <div className="min-w-0 flex-1 space-y-3">
          <div className="space-y-1">
            <p className="text-sm font-semibold tracking-tight">
              Network mismatch detected
            </p>
            <p className="text-xs text-muted-foreground">
              Switch your wallet network to continue — this warning cannot be hidden.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <div className="rounded-lg border border-border/50 bg-background/40 px-3 py-2">
              <p className="text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                Wallet
              </p>
              <p className="font-mono text-sm font-semibold uppercase">
                {formatNetworkLabel(walletNetwork)}
              </p>
            </div>
            <ArrowRight className="h-4 w-4 text-signal" aria-hidden />
            <div className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-2">
              <p className="text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                App
              </p>
              <p className="font-mono text-sm font-semibold uppercase text-primary">
                {formatNetworkLabel(network)}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {canUseWalletNetwork && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setNetwork(walletNetwork)}
                className="h-9 border-signal/35 bg-background/50 text-xs"
              >
                Use wallet network
              </Button>
            )}
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
                  How to switch network
                  <ExternalLink className="h-3 w-3" />
                </a>
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={disconnect}
              className="h-9 text-xs text-muted-foreground hover:text-foreground"
            >
              Disconnect wallet
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
