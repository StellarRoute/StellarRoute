'use client';

import { useEffect, useState } from 'react';
import { AlertTriangle, ArrowRight, ExternalLink, Loader2, Radio } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useWallet } from '@/components/providers/wallet-provider';
import { useHealth } from '@/hooks/useApi';
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

function ApiReachability({
  loading,
  ok,
  error,
}: {
  loading: boolean;
  ok: boolean;
  error: Error | null;
}) {
  if (loading && !ok && !error) {
    return (
      <span className="inline-flex items-center gap-1.5 text-muted-foreground">
        <Loader2 className="h-3 w-3 animate-spin" aria-hidden />
        Checking API…
      </span>
    );
  }
  if (ok) {
    return (
      <span className="inline-flex items-center gap-1.5 text-emerald-700 dark:text-emerald-400">
        <Radio className="h-3 w-3" aria-hidden />
        API reachable
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 text-signal">
      <AlertTriangle className="h-3 w-3" aria-hidden />
      API unreachable
    </span>
  );
}

/**
 * Persistent network strip: quiet status when matched;
 * roomy mismatch callout when wallet and app disagree.
 * Also surfaces API reachability for production wiring (#1036).
 */
export function NetworkStatusBanner() {
  const { network, networkMismatch, walletNetwork, walletId, disconnect, setNetwork } =
    useWallet();
  const { data: health, loading: healthLoading, error: healthError } = useHealth(30_000);
  const [reachabilityMounted, setReachabilityMounted] = useState(false);

  useEffect(() => {
    setReachabilityMounted(true);
  }, []);

  const apiOk = Boolean(health) && !healthError;
  const reachabilityLoading = !reachabilityMounted || healthLoading;
  const reachabilityError =
    reachabilityMounted && healthError ? new Error(healthError.message) : null;

  const walletDocsUrl = walletId ? WALLET_DOCS[walletId] : null;
  const canUseWalletNetwork =
    walletNetwork !== null && isNetworkAllowed(walletNetwork);

  if (networkMismatch) {
    return (
      <div
        data-testid="network-status-banner"
        role="alert"
        aria-live="assertive"
        className="w-full border-b border-signal/30 bg-signal/10"
      >
        <div className="container mx-auto max-w-7xl space-y-4 px-4 py-4 sm:px-6 lg:px-8">
          <div className="flex items-start gap-3">
            <span className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-signal/20 text-signal">
              <AlertTriangle className="h-4 w-4" aria-hidden />
            </span>
            <div className="space-y-1">
              <p className="text-sm font-semibold tracking-tight text-foreground">
                Networks don&apos;t match
              </p>
              <p className="max-w-xl text-sm text-muted-foreground">
                Signing is paused until your wallet is on the same network as the app.
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3 pl-0 sm:pl-12">
            <div className="rounded-xl border border-border/50 bg-background/45 px-4 py-2.5">
              <p className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                Wallet
              </p>
              <p className="font-mono text-sm font-semibold uppercase text-foreground">
                {formatNetworkLabel(walletNetwork)}
              </p>
            </div>
            <ArrowRight className="h-4 w-4 text-signal" aria-hidden />
            <div className="rounded-xl border border-primary/30 bg-primary/10 px-4 py-2.5">
              <p className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                App needs
              </p>
              <p className="font-mono text-sm font-semibold uppercase text-primary">
                {formatNetworkLabel(network)}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 pl-0 sm:pl-12">
            {canUseWalletNetwork && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setNetwork(walletNetwork)}
                className="h-10 border-signal/40 bg-background/60"
              >
                Use wallet network
              </Button>
            )}
            {walletDocsUrl && (
              <Button
                variant="outline"
                size="sm"
                asChild
                className="h-10 border-border/60 bg-background/60"
              >
                <a
                  href={walletDocsUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1.5"
                >
                  How to switch
                  <ExternalLink className="h-3.5 w-3.5" />
                </a>
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={disconnect}
              className="h-10 text-muted-foreground hover:text-foreground"
            >
              Disconnect
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="network-status-banner"
      role="status"
      aria-live="polite"
      className={cn(
        'w-full border-b px-4 py-2 text-xs font-medium',
        network === 'mainnet'
          ? 'border-signal/30 bg-signal/10 text-foreground'
          : 'border-primary/25 bg-primary/10 text-foreground'
      )}
    >
      <div className="container mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3 sm:px-2 lg:px-4">
        <p>
          Active network:{' '}
          <span className="font-semibold uppercase tracking-wide">
            {network === 'mainnet' ? 'Mainnet' : 'Testnet'}
          </span>
        </p>
        <div className="flex flex-wrap items-center gap-4">
          <ApiReachability
            loading={reachabilityLoading}
            ok={reachabilityMounted && apiOk}
            error={reachabilityError}
          />
          <p className="hidden text-muted-foreground sm:block">
            {network === 'mainnet'
              ? 'Public Stellar network — real funds'
              : 'SDF testnet — safe for trial swaps'}
          </p>
        </div>
      </div>
    </div>
  );
}
