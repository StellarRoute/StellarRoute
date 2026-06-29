'use client';

import React, { useEffect, useMemo, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { SupportedWallet, AvailableWallet, WalletNetwork } from '@/lib/wallet/types';
import {
  getAllowedNetworks,
  isNetworkAllowed,
  normalizeAppNetwork,
  type AppNetwork,
} from '@/lib/network-policy';
import { AlertCircle, CheckCircle, Loader2, AlertTriangle, ExternalLink } from 'lucide-react';
import { STELLAR_NETWORK } from '@/lib/constants';

export type OnboardingStep =
  | 'welcome'
  | 'select-network'
  | 'select-wallet'
  | 'connecting'
  | 'success'
  | 'error'
  | 'network-mismatch';

export interface WalletConnectionOnboardingProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  availableWallets: AvailableWallet[];
  isLoading: boolean;
  error: string | null;
  onConnect: (walletId: SupportedWallet) => Promise<void>;
  appNetwork: WalletNetwork;
  walletNetwork: string | null;
  onNetworkSelection?: (network: WalletNetwork) => void;
}

const NETWORK_LABELS: Record<AppNetwork, string> = {
  testnet: 'Testnet',
  mainnet: 'Mainnet',
};

export function WalletConnectionOnboarding({
  open,
  onOpenChange,
  availableWallets,
  isLoading,
  error,
  onConnect,
  appNetwork,
  walletNetwork,
  onNetworkSelection,
}: WalletConnectionOnboardingProps) {
  const allowedNetworks = useMemo(() => getAllowedNetworks(), []);
  const [step, setStep] = useState<OnboardingStep>('welcome');
  const [selectedWallet, setSelectedWallet] = useState<SupportedWallet | null>(null);
  const [selectedNetwork, setSelectedNetwork] = useState<AppNetwork>(() => {
    const normalizedApp = normalizeAppNetwork(appNetwork);
    return normalizedApp && allowedNetworks.includes(normalizedApp)
      ? normalizedApp
      : allowedNetworks[0];
  });
  const [connectionError, setConnectionError] = useState<string | null>(error);

  useEffect(() => {
    const normalizedApp = normalizeAppNetwork(appNetwork);
    if (normalizedApp && allowedNetworks.includes(normalizedApp)) {
      setSelectedNetwork(normalizedApp);
    }
  }, [appNetwork, allowedNetworks]);

  useEffect(() => {
    if (step !== 'connecting' || !walletNetwork) {
      return;
    }

    const mismatch =
      normalizeAppNetwork(walletNetwork) !== normalizeAppNetwork(selectedNetwork);
    setStep(mismatch ? 'network-mismatch' : 'success');
  }, [step, walletNetwork, selectedNetwork]);

  const handleNetworkChoice = (network: AppNetwork) => {
    setSelectedNetwork(network);
    onNetworkSelection?.(network);
    setStep('select-wallet');
  };

  const handleContinueFromWelcome = () => {
    if (allowedNetworks.length > 1) {
      setStep('select-network');
      return;
    }
    setStep('select-wallet');
  };

  const handleWalletSelect = async (wallet: AvailableWallet) => {
    if (!wallet.installed) {
      window.open(
        wallet.id === 'freighter'
          ? 'https://www.freighter.app/'
          : 'https://wallet.xbull.app/',
        '_blank',
      );
      return;
    }

    onNetworkSelection?.(selectedNetwork);
    setSelectedWallet(wallet.id as SupportedWallet);
    setConnectionError(null);
    setStep('connecting');

    try {
      await onConnect(wallet.id as SupportedWallet);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Connection failed. Please try again.';
      setConnectionError(errorMessage);
      setStep('error');
    }
  };

  const handleUseWalletNetwork = () => {
    if (!walletNetwork || !isNetworkAllowed(walletNetwork)) {
      return;
    }
    onNetworkSelection?.(walletNetwork);
    setStep('success');
  };

  const handleRetry = () => {
    if (selectedWallet) {
      setConnectionError(null);
      setStep('connecting');
      const wallet = availableWallets.find((w) => w.id === selectedWallet);
      if (wallet) {
        void handleWalletSelect(wallet);
      }
    } else {
      setStep('select-wallet');
    }
  };

  const resetFlow = () => {
    setStep('welcome');
    setSelectedWallet(null);
    setConnectionError(null);
  };

  const handleClose = () => {
    if (['welcome', 'success', 'error'].includes(step)) {
      onOpenChange(false);
      resetFlow();
    }
  };

  const handleNetworkMismatchClose = () => {
    onOpenChange(false);
    resetFlow();
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[425px] md:max-w-[600px]">
        {step === 'welcome' && (
          <>
            <DialogHeader>
              <DialogTitle>Connect Your Wallet</DialogTitle>
              <DialogDescription>
                Get started with StellarRoute by connecting your Stellar wallet
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-4">
              <div className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  To begin trading on StellarRoute, you&apos;ll need to connect your Stellar wallet.
                  We support:
                </p>
                <ul className="space-y-2 text-sm">
                  <li className="flex items-start gap-3">
                    <span className="text-primary mt-0.5">✓</span>
                    <span>
                      <strong>Freighter</strong> - A browser extension wallet for Stellar
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-primary mt-0.5">✓</span>
                    <span>
                      <strong>xBull</strong> - A web-based Stellar wallet
                    </span>
                  </li>
                </ul>
              </div>

              <Alert>
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  <strong>Why do we ask for wallet connection?</strong>
                  <p className="mt-1 text-xs">
                    We use your wallet connection to display balances, execute trades with your
                    permission, and manage transaction history. We never access your private keys.
                  </p>
                </AlertDescription>
              </Alert>

              <div className="flex gap-2 pt-4">
                <Button variant="outline" onClick={handleClose} className="flex-1">
                  Cancel
                </Button>
                <Button onClick={handleContinueFromWelcome} className="flex-1">
                  Continue
                </Button>
              </div>
            </div>
          </>
        )}

        {step === 'select-network' && (
          <>
            <DialogHeader>
              <DialogTitle>Select Network</DialogTitle>
              <DialogDescription>
                Choose the Stellar network you want to use in StellarRoute
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="grid gap-3">
                {allowedNetworks.map((network) => (
                  <button
                    key={network}
                    type="button"
                    onClick={() => handleNetworkChoice(network)}
                    className={`relative p-4 rounded-lg border-2 transition-all text-left ${
                      selectedNetwork === network
                        ? 'border-primary bg-accent'
                        : 'border-border hover:border-primary hover:bg-accent'
                    }`}
                  >
                    <h4 className="font-semibold">{NETWORK_LABELS[network]}</h4>
                    <p className="text-sm text-muted-foreground">
                      Use Stellar {NETWORK_LABELS[network]} for quotes and swaps
                    </p>
                  </button>
                ))}
              </div>
              <div className="flex gap-2 pt-4">
                <Button variant="outline" onClick={() => setStep('welcome')} className="flex-1">
                  Back
                </Button>
              </div>
            </div>
          </>
        )}

        {step === 'select-wallet' && (
          <>
            <DialogHeader>
              <DialogTitle>Select Your Wallet</DialogTitle>
              <DialogDescription>
                Connecting on {NETWORK_LABELS[selectedNetwork] ?? appNetwork}. Choose
                which Stellar wallet you&apos;d like to connect.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              {availableWallets.length > 0 ? (
                <div className="grid gap-3">
                  {availableWallets.map((wallet) => (
                    <button
                      key={wallet.id}
                      type="button"
                      onClick={() => handleWalletSelect(wallet)}
                      disabled={isLoading}
                      className={`relative p-4 rounded-lg border-2 transition-all text-left ${
                        !wallet.installed
                          ? 'border-dashed border-muted-foreground/50 bg-muted/30 hover:border-primary hover:bg-muted/50'
                          : 'border-border hover:border-primary hover:bg-accent'
                      } disabled:opacity-50`}
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <h4 className="font-semibold">{wallet.label}</h4>
                          <p className="text-sm text-muted-foreground">
                            {wallet.installed ? 'Detected on your device' : 'Not installed'}
                          </p>
                          {wallet.id === 'xbull' && (
                            <p className="text-xs text-muted-foreground mt-1">
                              Transaction signing is supported on testnet only.
                            </p>
                          )}
                        </div>
                        {!wallet.installed && (
                          <ExternalLink className="h-4 w-4 text-muted-foreground" />
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              ) : (
                <Alert>
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription>
                    <p className="font-medium">No Supported Wallet Found</p>
                  </AlertDescription>
                </Alert>
              )}

              <div className="flex gap-2 pt-4">
                <Button
                  variant="outline"
                  onClick={() =>
                    setStep(allowedNetworks.length > 1 ? 'select-network' : 'welcome')
                  }
                  className="flex-1"
                >
                  Back
                </Button>
              </div>
            </div>
          </>
        )}

        {step === 'connecting' && (
          <>
            <DialogHeader>
              <DialogTitle>
                Connecting {selectedWallet === 'freighter' ? 'Freighter' : 'xBull'}
              </DialogTitle>
              <DialogDescription>
                Please approve the connection in your wallet
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-8 flex flex-col items-center">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <div className="text-center space-y-2">
                <p className="font-medium">Waiting for approval...</p>
              </div>
            </div>
          </>
        )}

        {step === 'success' && (
          <>
            <DialogHeader>
              <DialogTitle>Wallet Connected!</DialogTitle>
              <DialogDescription>
                Your wallet is connected on {String(appNetwork)}
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-8 flex flex-col items-center">
              <CheckCircle className="h-12 w-12 text-green-500" />
              <div className="text-center space-y-2">
                <p className="font-medium text-green-700">Connection Successful</p>
                <p className="text-sm text-muted-foreground">
                  You&apos;re ready to start trading on StellarRoute
                </p>
              </div>
              <Button onClick={handleClose} className="w-full">
                Start Trading
              </Button>
            </div>
          </>
        )}

        {step === 'error' && (
          <>
            <DialogHeader>
              <DialogTitle>Connection Failed</DialogTitle>
              <DialogDescription>
                We encountered an issue connecting your wallet
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-4">
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{connectionError}</AlertDescription>
              </Alert>

              <div className="bg-muted/50 p-4 rounded-lg space-y-2 text-sm">
                <p className="font-medium">Troubleshooting tips:</p>
                <ul className="space-y-1 text-muted-foreground list-inside list-disc">
                  <li>Ensure your wallet extension/app is enabled</li>
                  <li>Try refreshing the page</li>
                  <li>Check that you&apos;re using the correct network</li>
                  <li>Clear your browser cache and try again</li>
                </ul>
              </div>

              <div className="flex gap-2">
                <Button variant="outline" onClick={() => setStep('select-wallet')} className="flex-1">
                  Try Different Wallet
                </Button>
                <Button onClick={handleRetry} className="flex-1">
                  Retry
                </Button>
              </div>
            </div>
          </>
        )}

        {step === 'network-mismatch' && (
          <>
            <DialogHeader>
              <DialogTitle>Network Mismatch</DialogTitle>
              <DialogDescription>
                Your wallet is on a different network than StellarRoute
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-4">
              <Alert>
                <AlertTriangle className="h-4 w-4" />
                <AlertDescription>
                  <p className="font-medium mb-2">Wallet network: {walletNetwork || 'Unknown'}</p>
                  <p className="text-sm mb-2">
                    StellarRoute is set to <strong>{appNetwork}</strong>.
                  </p>
                  <div className="bg-background p-3 rounded border text-xs font-mono">
                    Wallet: {walletNetwork} | App: {appNetwork}
                  </div>
                </AlertDescription>
              </Alert>

              <div className="flex flex-col gap-2 sm:flex-row">
                <Button
                  variant="outline"
                  onClick={() => setStep('select-wallet')}
                  className="flex-1"
                >
                  Try Again
                </Button>
                {walletNetwork && isNetworkAllowed(walletNetwork) && (
                  <Button onClick={handleUseWalletNetwork} className="flex-1">
                    Use wallet network
                  </Button>
                )}
                <Button onClick={handleNetworkMismatchClose} className="flex-1">
                  Close
                </Button>
              </div>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
