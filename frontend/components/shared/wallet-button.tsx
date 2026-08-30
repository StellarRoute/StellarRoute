'use client';

import { useEffect, useState } from 'react';
import { Check, ChevronDown, Copy, LogOut, QrCode, RefreshCw } from 'lucide-react';
import { QRCodeSVG } from 'qrcode.react';
import { useWallet } from '@/components/providers/wallet-provider';
import { useWalletOnboarding } from '@/hooks/useWalletOnboarding';
import { WalletConnectionOnboarding } from '@/components/modals/WalletConnectionOnboarding';
import { AccountSwitcher } from './account-switcher';
import type { SupportedWallet, WalletNetwork } from '@/lib/wallet/types';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

function formatShortAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

export function WalletButton() {
  const [showQrCode, setShowQrCode] = useState(false);
  const [copied, setCopied] = useState(false);
  const [showOnboardingModal, setShowOnboardingModal] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);

  const {
    address,
    isConnected,
    network,
    walletNetwork,
    availableWallets,
    isLoading,
    error,
    connect,
    disconnect,
    setNetwork,
    refreshAccount,
    refreshWallets,
    walletId,
  } = useWallet();

  const {
    showOnboarding,
    isFirstConnection,
    markOnboardingAsCompleted,
    markOnboardingAsSeenAndOpened,
  } = useWalletOnboarding({
    isConnected,
  });

  useEffect(() => {
    if (showOnboarding && isFirstConnection && !showOnboardingModal) {
      setShowOnboardingModal(true);
      markOnboardingAsSeenAndOpened();
    }
  }, [
    showOnboarding,
    isFirstConnection,
    showOnboardingModal,
    markOnboardingAsSeenAndOpened,
  ]);

  const handleOnboardingConnect = async (walletId: SupportedWallet) => {
    try {
      await connect(walletId);
      markOnboardingAsCompleted();
    } catch (err) {
      throw err;
    }
  };

  const handleNetworkSelection = (nextNetwork: WalletNetwork) => {
    setNetwork(nextNetwork);
  };

  const copyAddress = async () => {
    if (!address) return;
    try {
      await navigator.clipboard.writeText(address);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch (err) {
      console.error('Failed to copy address:', err);
    }
  };

  const handleRefresh = async () => {
    if (!walletId) return;
    setIsRefreshing(true);
    try {
      await refreshAccount();
    } catch (err) {
      console.error('Failed to refresh account:', err);
    } finally {
      setIsRefreshing(false);
    }
  };

  if (!isConnected) {
    return (
      <>
        <Button
          id="wallet-button"
          onClick={() => setShowOnboardingModal(true)}
          className="min-h-[44px]"
        >
          Connect Wallet
        </Button>

        <WalletConnectionOnboarding
          open={showOnboardingModal}
          onOpenChange={setShowOnboardingModal}
          availableWallets={availableWallets}
          isLoading={isLoading}
          error={error?.message ?? null}
          onConnect={handleOnboardingConnect}
          appNetwork={network}
          walletNetwork={walletNetwork}
          onNetworkSelection={handleNetworkSelection}
          onRefreshWallets={refreshWallets}
        />
      </>
    );
  }

  return (
    <div className="relative flex items-center gap-2">
      <AccountSwitcher
        onAccountChange={() => {
          setShowQrCode(false);
        }}
      />

      <DropdownMenu
        onOpenChange={(open) => {
          if (!open) setShowQrCode(false);
        }}
      >
        <DropdownMenuTrigger asChild>
          <Button
            id="wallet-button"
            variant="outline"
            className="h-10 gap-2 rounded-full border-border/60 bg-background/60 px-3 font-mono text-sm"
            aria-label={`Wallet ${address ? formatShortAddress(address) : 'connected'}`}
          >
            <span className="h-2 w-2 rounded-full bg-primary" aria-hidden />
            {address ? formatShortAddress(address) : 'Connected'}
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
          </Button>
        </DropdownMenuTrigger>

        <DropdownMenuContent align="end" className="w-72 p-2">
          <DropdownMenuLabel className="px-2 py-1.5 text-xs font-normal text-muted-foreground">
            Connected wallet
          </DropdownMenuLabel>
          <div className="rounded-lg border border-border/50 bg-muted/20 px-3 py-2.5">
            <p className="break-all font-mono text-xs leading-relaxed text-foreground">
              {address}
            </p>
          </div>

          <DropdownMenuSeparator className="my-2" />

          <DropdownMenuItem onClick={copyAddress} className="gap-2 cursor-pointer">
            {copied ? <Check className="h-4 w-4 text-primary" /> : <Copy className="h-4 w-4" />}
            {copied ? 'Copied' : 'Copy address'}
          </DropdownMenuItem>
          <DropdownMenuItem
            onSelect={(event) => {
              event.preventDefault();
              setShowQrCode((prev) => !prev);
            }}
            className="gap-2 cursor-pointer"
          >
            <QrCode className="h-4 w-4" />
            {showQrCode ? 'Hide QR code' : 'Show QR code'}
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={handleRefresh}
            disabled={isRefreshing}
            className="gap-2 cursor-pointer"
          >
            <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`} />
            {isRefreshing ? 'Refreshing…' : 'Refresh account'}
          </DropdownMenuItem>

          {showQrCode && address && (
            <div className="mt-2 flex flex-col items-center gap-2 rounded-lg border border-border/50 bg-background p-3">
              <div className="rounded-md bg-white p-2">
                <QRCodeSVG value={address} size={140} level="H" includeMargin />
              </div>
            </div>
          )}

          <DropdownMenuSeparator className="my-2" />

          <DropdownMenuItem
            onClick={disconnect}
            className="gap-2 cursor-pointer text-destructive focus:text-destructive"
          >
            <LogOut className="h-4 w-4" />
            Disconnect
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {error && (
        <p className="absolute top-full right-0 mt-2 max-w-xs text-xs text-destructive">
          {error.message}
        </p>
      )}
    </div>
  );
}
