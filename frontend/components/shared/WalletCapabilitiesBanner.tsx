'use client';

import { ShieldX, ShieldCheck, RefreshCw, ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useWallet } from '@/components/providers/wallet-provider';
import { cn } from '@/lib/utils';
import { useCallback } from 'react';

const WALLET_DOCS: Record<string, string> = {
  freighter: 'https://docs.freighter.app/docs/guide/gettingStarted',
  xbull: 'https://xbull.app/docs',
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

function getCapabilityIcon(capability: string): typeof ShieldCheck {
  switch (capability) {
    case 'sign_transaction':
      return ShieldX;
    case 'view_address':
      return ShieldX;
    case 'view_network':
      return ShieldX;
    case 'request_access':
      return ShieldX;
    default:
      return ShieldCheck;
  }
}

export function WalletCapabilitiesBanner({ className }: WalletCapabilitiesBannerProps) {
  const { capabilities, walletId, refreshCapabilities } = useWallet();

  const handleRefresh = useCallback(() => {
    void refreshCapabilities();
  }, [refreshCapabilities]);

  if (!capabilities) return null;

  const denied = capabilities.statuses.filter((s) => !s.allowed);
  if (denied.length === 0) return null;

  const walletDocsUrl = walletId ? WALLET_DOCS[walletId] : null;

  return (
    <Alert
      variant="destructive"
      className={cn(
        'relative border-red-500/50 bg-red-500/10 text-red-900 dark:text-red-100',
        className
      )}
      role="alert"
      aria-live="polite"
    >
      <ShieldX className="h-4 w-4 text-red-600 dark:text-red-400" />
      <AlertDescription className="flex flex-col gap-3 pr-8">
        <div className="text-sm font-medium">
          Wallet permissions required
        </div>
        <div className="flex flex-col gap-2 text-xs text-red-800 dark:text-red-200">
          {denied.map((status) => {
            const Icon = getCapabilityIcon(status.capability);
            return (
              <div
                key={status.capability}
                className="flex items-start gap-2"
              >
                <Icon className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                <div className="flex flex-col">
                  <span className="font-medium">
                    {getCapabilityLabel(status.capability)}
                  </span>
                  {status.reason && (
                    <span className="opacity-80">{status.reason}</span>
                  )}
                  {status.resolution && (
                    <span className="font-medium text-red-700 dark:text-red-300">
                      {status.resolution}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {walletDocsUrl && (
            <Button
              variant="outline"
              size="sm"
              asChild
              className="h-8 text-xs border-red-600/30 hover:bg-red-500/20"
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
            className="h-8 text-xs border-red-600/30 hover:bg-red-500/20"
          >
            <RefreshCw className="h-3 w-3 mr-1" />
            Check again
          </Button>
        </div>
      </AlertDescription>
    </Alert>
  );
}