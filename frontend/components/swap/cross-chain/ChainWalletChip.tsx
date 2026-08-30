'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { shortenAddress } from '@/lib/cross-chain/format';
import type { WalletChipBinding } from '@/lib/cross-chain/wallet-chip-types';
import { cn } from '@/lib/utils';
import { Loader2, Plug, Unplug, Wallet } from 'lucide-react';
import type { CrossChainWalletStoryState } from './crossChainStoryPresentation';

interface ChainWalletChipProps {
  binding?: WalletChipBinding | null;
  storyState?: CrossChainWalletStoryState;
  className?: string;
  disabled?: boolean;
}

export function ChainWalletChip({
  binding,
  storyState,
  className,
  disabled = false,
}: ChainWalletChipProps) {
  if (!binding) return null;
  return (
    <BoundWalletChip
      binding={binding}
      storyState={storyState}
      className={className}
      disabled={disabled}
    />
  );
}

function BoundWalletChip({
  binding,
  storyState,
  className,
  disabled,
}: {
  binding: WalletChipBinding;
  storyState?: CrossChainWalletStoryState;
  className?: string;
  disabled?: boolean;
}) {
  const [pickerOpen, setPickerOpen] = useState(false);

  const isConnecting = storyState === 'connecting' || binding.isConnecting;
  const isConnected =
    storyState === 'connected' ||
    (storyState === undefined && binding.isConnected);
  const networkMismatch =
    storyState === 'mismatch' ||
    (storyState === undefined && binding.networkMismatch);
  const unsupported = storyState === 'unsupported' || binding.unsupported;

  const showConnectCta =
    !binding.readOnly && !isConnected && !unsupported && !isConnecting;

  const statusLabel = unsupported
    ? 'Unsupported wallet'
    : binding.readOnly
      ? 'Recipient'
      : isConnecting
        ? 'Connecting'
        : networkMismatch
          ? 'Network mismatch'
          : isConnected
            ? 'Connected'
            : 'Disconnected';

  const openPicker = () => {
    if (!binding.readOnly) setPickerOpen(true);
  };

  return (
    <>
      {showConnectCta ? (
        <Button
          type="button"
          className={cn('min-h-11 w-full gap-2', className)}
          disabled={disabled}
          onClick={openPicker}
          aria-label={`Connect ${binding.chainLabel} wallet`}
          data-testid={binding.testId}
        >
          <Wallet className="h-4 w-4" aria-hidden />
          Connect {binding.chainShortLabel} wallet
        </Button>
      ) : (
        <button
          type="button"
          disabled={disabled || binding.readOnly}
          onClick={openPicker}
          className={cn(
            'flex min-h-11 items-center gap-2 rounded-xl border px-3 py-2 text-left transition-colors',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
            networkMismatch
              ? 'border-signal/40 bg-signal/10'
              : isConnected
                ? 'border-primary/35 bg-primary/10'
                : 'border-border/50 bg-background/40',
            (disabled || binding.readOnly) && 'opacity-70 cursor-default',
            className,
          )}
          aria-label={`${binding.chainLabel} wallet: ${statusLabel}`}
          data-testid={binding.testId}
        >
          <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted/50">
            {isConnecting ? (
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
            ) : (
              <Wallet className="h-4 w-4" aria-hidden />
            )}
          </span>
          <span className="min-w-0">
            <span className="block text-[10px] uppercase tracking-wide text-muted-foreground">
              {statusLabel}
            </span>
            <span className="block truncate font-mono text-xs font-semibold">
              {isConnected && binding.address
                ? shortenAddress(binding.address)
                : binding.chainShortLabel}
            </span>
          </span>
        </button>
      )}

      <Dialog open={pickerOpen} onOpenChange={setPickerOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Connect {binding.chainLabel} wallet</DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            {networkMismatch && (
              <p role="alert" className="text-sm text-signal">
                Wallet network does not match {binding.chainLabel}. Switch
                networks before signing.
              </p>
            )}
            {binding.availableWallets.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No wallet option available for this chain yet.
              </p>
            ) : (
              <ul className="space-y-2">
                {binding.availableWallets.map((wallet) => (
                  <li key={wallet.id}>
                    <Button
                      type="button"
                      variant="outline"
                      className="w-full justify-start gap-2 min-h-11"
                      disabled={!wallet.installed || isConnecting}
                      onClick={() => {
                        void binding
                          .onConnect(wallet.id)
                          .then(() => setPickerOpen(false));
                      }}
                    >
                      <Plug className="h-4 w-4" aria-hidden />
                      {wallet.label}
                      {!wallet.installed && (
                        <span className="text-muted-foreground">
                          {wallet.id === 'evm-walletconnect'
                            ? ' (not configured)'
                            : ' (not installed)'}
                        </span>
                      )}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
            {isConnected && binding.onDisconnect && (
              <Button
                type="button"
                variant="ghost"
                className="min-h-11 gap-2"
                onClick={() => void binding.onDisconnect?.()}
              >
                <Unplug className="h-4 w-4" aria-hidden />
                Disconnect
              </Button>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
