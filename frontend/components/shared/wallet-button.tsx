'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { useWallet } from '@/components/providers/wallet-provider';
import { useWalletOnboarding } from '@/hooks/useWalletOnboarding';
import { WalletConnectionOnboarding } from '@/components/modals/WalletConnectionOnboarding';
import { AccountSwitcher } from './account-switcher';
import { Button } from '@/components/ui/button';

const APP_NETWORK = 'TESTNET';

export function WalletButton() {
  const [showQrCode, setShowQrCode] = useState(false);
  const [showOnboardingModal, setShowOnboardingModal] = useState(false);
  const [walletNetworkForOnboarding, setWalletNetworkForOnboarding] = useState<string | null>(null);

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
  } = useWallet();

  const shortAddress = address
    ? `${address.slice(0, 4)}...${address.slice(-4)}`
    : '';

  const copyAddress = async () => {
    if (address) {
      try {
        await navigator.clipboard.writeText(address);
      } catch (err) {
        console.error('Failed to copy address:', err);
      }
    }
  };

  const {
    showOnboarding,
    isFirstConnection,
    markOnboardingAsCompleted,
    markOnboardingAsSeenAndOpened,
  } = useWalletOnboarding({
    isConnected,
  });

  const mismatch =
    walletNetwork &&
    walletNetwork.toUpperCase() !== APP_NETWORK.toUpperCase();

  // Auto-open onboarding for first-time users
  useEffect(() => {
    if (showOnboarding && isFirstConnection && !showOnboardingModal) {
      setShowOnboardingModal(true);
      markOnboardingAsSeenAndOpened();
    }
  }, [showOnboarding, isFirstConnection, showOnboardingModal, markOnboardingAsSeenAndOpened]);

  const handleOnboardingConnect = async (walletId: any) => {
    try {
      await connect(walletId);
      setWalletNetworkForOnboarding(walletNetwork ?? null);
      markOnboardingAsCompleted();
    } catch (err) {
      // Error will be shown in onboarding modal
      throw err;
    }
  };

  if (!isConnected) {
    return (
      <>
        <Button
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
          walletNetwork={walletNetworkForOnboarding}
        />
      </>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <AccountSwitcher
        onAccountChange={(newAddress) => {
          console.log('Account changed to:', newAddress);
          setShowQrCode(false);
        }}
      />

      <div className="flex items-center gap-2">
        <span className="rounded-md border px-3 py-2 text-sm font-mono bg-muted/20">
          {shortAddress}
        </span>

        <button
          onClick={() => setShowQrCode(!showQrCode)}
          className={`rounded-md border px-3 py-2 text-sm font-medium transition-all duration-200 cursor-pointer ${
            showQrCode
              ? 'bg-primary text-primary-foreground border-primary shadow-sm'
              : 'hover:bg-accent hover:text-accent-foreground bg-background'
          }`}
          title={showQrCode ? 'Hide QR Code' : 'Show QR Code'}
          aria-expanded={showQrCode}
        >
          {showQrCode ? 'Hide QR' : 'Show QR'}
        </button>

        <button
          onClick={copyAddress}
          className="rounded-md border px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground bg-background transition-colors cursor-pointer"
        >
          Copy
        </button>

        <button
          onClick={disconnect}
          className="rounded-md border px-3 py-2 text-sm hover:bg-destructive hover:text-destructive-foreground bg-background transition-colors cursor-pointer"
        >
          Disconnect
        </button>
      </div>

      {showQrCode && address && (
        <div className="flex flex-col items-center gap-3 rounded-xl border bg-card p-4 text-card-foreground shadow-md transition-all duration-300 animate-in fade-in slide-in-from-top-2">
          <div className="rounded-lg bg-white p-3 shadow-inner border border-border flex items-center justify-center">
            <QRCodeSVG
              value={address}
              size={160}
              level="H"
              includeMargin={true}
            />
          </div>
          <div className="flex flex-col items-center gap-1 text-center w-full max-w-[220px]">
            <span className="text-[10px] uppercase tracking-wider font-semibold text-muted-foreground">
              Public Address
            </span>
            <span className="text-xs font-mono select-all break-all text-foreground/80 leading-relaxed">
              {address}
            </span>
          </div>
        </div>
      )}

      <div className="text-sm text-muted-foreground">
        Wallet network: {walletNetwork ?? network ?? 'Unknown'}
      </div>

      {mismatch && (
        <div className="text-sm text-yellow-600 font-medium">
          Network mismatch: app is {APP_NETWORK}, wallet is {walletNetwork}
        </div>
      )}

      {error && <p className="text-sm text-destructive">{error.message}</p>}
    </div>
  );
}
