'use client';

import React, { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { SupportedWallet, AvailableWallet } from '@/lib/wallet/types';
import { AlertCircle, CheckCircle, Loader2, AlertTriangle, ExternalLink } from 'lucide-react';

export type OnboardingStep = 'welcome' | 'select-wallet' | 'connecting' | 'success' | 'error' | 'network-mismatch';

export interface WalletConnectionOnboardingProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  availableWallets: AvailableWallet[];
  isLoading: boolean;
  error: string | null;
  onConnect: (walletId: SupportedWallet) => Promise<void>;
  walletNetwork: string | null;
  onNetworkSelection?: (network: string) => void;
}

const SUPPORTED_NETWORKS = ['testnet', 'mainnet'];
const APP_NETWORK = 'testnet';

export function WalletConnectionOnboarding({
  open,
  onOpenChange,
  availableWallets,
  isLoading,
  error,
  onConnect,
  walletNetwork,
  onNetworkSelection,
}: WalletConnectionOnboardingProps) {
  const [step, setStep] = useState<OnboardingStep>('welcome');
  const [selectedWallet, setSelectedWallet] = useState<SupportedWallet | null>(null);
  const [selectedNetwork, setSelectedNetwork] = useState<string>(APP_NETWORK);
  const [connectionError, setConnectionError] = useState<string | null>(error);

  // Determine if we should show network mismatch after successful connection
  const showNetworkMismatch = step === 'success' && walletNetwork && walletNetwork.toLowerCase() !== APP_NETWORK.toLowerCase();

  const handleWalletSelect = async (wallet: AvailableWallet) => {
    if (!wallet.installed) {
      // User needs to install wallet
      window.open(
        wallet.id === 'freighter'
          ? 'https://www.freighter.app/'
          : 'https://wallet.xbull.app/',
        '_blank'
      );
      return;
    }

    setSelectedWallet(wallet.id as SupportedWallet);
    setConnectionError(null);
    setStep('connecting');

    try {
      await onConnect(wallet.id as SupportedWallet);
      setStep(showNetworkMismatch ? 'network-mismatch' : 'success');
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Connection failed. Please try again.';
      setConnectionError(errorMessage);
      setStep('error');
    }
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

  const handleClose = () => {
    // Allow closing in welcome, success, and error states
    if (['welcome', 'success', 'error'].includes(step)) {
      onOpenChange(false);
      // Reset on close
      setStep('welcome');
      setSelectedWallet(null);
      setConnectionError(null);
    }
  };

  const handleNetworkMismatchClose = () => {
    onOpenChange(false);
    setStep('welcome');
    setSelectedWallet(null);
    setConnectionError(null);
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
                  To begin trading on StellarRoute, you'll need to connect your Stellar wallet.
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
                    We use your wallet connection to:
                  </p>
                  <ul className="mt-1 space-y-1 text-xs list-inside list-disc">
                    <li>Display your account balance</li>
                    <li>Execute trades with your permission</li>
                    <li>Manage your transaction history</li>
                  </ul>
                  <p className="mt-2 text-xs">
                    We never access your private keys. All transactions require your explicit approval.
                  </p>
                </AlertDescription>
              </Alert>

              <div className="flex gap-2 pt-4">
                <Button variant="outline" onClick={handleClose} className="flex-1">
                  Cancel
                </Button>
                <Button onClick={() => setStep('select-wallet')} className="flex-1">
                  Continue
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
                Choose which Stellar wallet you'd like to connect
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              {availableWallets.length > 0 ? (
                <div className="grid gap-3">
                  {availableWallets.map((wallet) => (
                    <button
                      key={wallet.id}
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
                        </div>
                        {!wallet.installed && (
                          <ExternalLink className="h-4 w-4 text-muted-foreground" />
                        )}
                      </div>
                      {!wallet.installed && (
                        <p className="text-xs text-muted-foreground mt-2">
                          Click to install
                        </p>
                      )}
                    </button>
                  ))}
                </div>
              ) : (
                <Alert>
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription>
                    <p className="font-medium">No Supported Wallet Found</p>
                    <p className="text-sm mt-2">
                      Please install one of the supported wallets:
                    </p>
                    <div className="flex gap-2 mt-4">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() =>
                          window.open('https://www.freighter.app/', '_blank')
                        }
                      >
                        Install Freighter
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() =>
                          window.open('https://wallet.xbull.app/', '_blank')
                        }
                      >
                        Install xBull
                      </Button>
                    </div>
                  </AlertDescription>
                </Alert>
              )}

              <div className="flex gap-2 pt-4">
                <Button
                  variant="outline"
                  onClick={() => setStep('welcome')}
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
              <DialogTitle>Connecting {selectedWallet === 'freighter' ? 'Freighter' : 'xBull'}</DialogTitle>
              <DialogDescription>
                Please approve the connection in your wallet
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-8 flex flex-col items-center">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <div className="text-center space-y-2">
                <p className="font-medium">Waiting for approval...</p>
                <p className="text-sm text-muted-foreground">
                  A popup or notification should appear in your wallet. Please review and approve the connection request.
                </p>
              </div>
            </div>
          </>
        )}

        {step === 'success' && (
          <>
            <DialogHeader>
              <DialogTitle>Wallet Connected!</DialogTitle>
              <DialogDescription>
                Your {selectedWallet === 'freighter' ? 'Freighter' : 'xBull'} wallet is now connected
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-8 flex flex-col items-center">
              <CheckCircle className="h-12 w-12 text-green-500" />
              <div className="text-center space-y-2">
                <p className="font-medium text-green-700">Connection Successful</p>
                <p className="text-sm text-muted-foreground">
                  You're ready to start trading on StellarRoute
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
                  <li>Check that you're using the correct network</li>
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
                Your wallet is on a different network
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-4">
              <Alert>
                <AlertTriangle className="h-4 w-4" />
                <AlertDescription>
                  <p className="font-medium mb-2">Wallet Network: {walletNetwork || 'Unknown'}</p>
                  <p className="text-sm mb-2">
                    StellarRoute is currently set to use the <strong>{APP_NETWORK}</strong> network.
                  </p>
                  <p className="text-sm mb-4">
                    To continue, please switch your wallet to the {APP_NETWORK} network, or you can proceed at your own risk.
                  </p>
                  <div className="bg-background p-3 rounded border text-xs font-mono">
                    Wallet: {walletNetwork} | App: {APP_NETWORK}
                  </div>
                </AlertDescription>
              </Alert>

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => setStep('select-wallet')}
                  className="flex-1"
                >
                  Try Again
                </Button>
                <Button onClick={handleNetworkMismatchClose} className="flex-1">
                  Proceed Anyway
                </Button>
              </div>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
