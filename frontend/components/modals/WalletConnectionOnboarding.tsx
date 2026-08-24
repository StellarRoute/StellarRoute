'use client';

import React, { useEffect, useMemo, useRef, useState } from 'react';
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
import { WALLET_INSTALL_URLS, WALLET_LABELS } from '@/lib/wallet';
import {
  getAllowedNetworks,
  isNetworkAllowed,
  normalizeAppNetwork,
  type AppNetwork,
} from '@/lib/network-policy';
import { AlertCircle, CheckCircle, Loader2, AlertTriangle, ExternalLink, RefreshCw } from 'lucide-react';

const CONNECT_TIMEOUT_MS = 90_000;
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
  onRefreshWallets?: () => Promise<void>;
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
  onRefreshWallets,
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
  const [isRefreshingWallets, setIsRefreshingWallets] = useState(false);
  const connectAttemptRef = useRef(0);
  const dismissedRef = useRef(false);

  useEffect(() => {
    const normalizedApp = normalizeAppNetwork(appNetwork);
    if (normalizedApp && allowedNetworks.includes(normalizedApp)) {
      setSelectedNetwork(normalizedApp);
    }
  }, [appNetwork, allowedNetworks]);

  useEffect(() => {
    if (step !== 'connecting' || !walletNetwork || dismissedRef.current) {
      return;
    }

    const mismatch =
      normalizeAppNetwork(walletNetwork) !== normalizeAppNetwork(selectedNetwork);
    setStep(mismatch ? 'network-mismatch' : 'success');
  }, [step, walletNetwork, selectedNetwork]);

  // If Freighter/extension never responds, leave the spinner and show an error.
  useEffect(() => {
    if (step !== 'connecting') {
      return;
    }

    const attempt = connectAttemptRef.current;
    const timer = window.setTimeout(() => {
      if (dismissedRef.current || connectAttemptRef.current !== attempt) {
        return;
      }
      setConnectionError(
        'Wallet did not respond in time. Open Freighter (or your extension), approve the request, or cancel and try again.'
      );
      setStep('error');
    }, CONNECT_TIMEOUT_MS);

    return () => window.clearTimeout(timer);
  }, [step, selectedWallet]);

  // Re-detect extensions when the picker is open — they often inject after page load.
  useEffect(() => {
    if (!open || step !== 'select-wallet' || !onRefreshWallets) {
      return;
    }

    let cancelled = false;
    const refresh = async (showSpinner: boolean) => {
      if (cancelled) return;
      if (showSpinner) setIsRefreshingWallets(true);
      try {
        await onRefreshWallets();
      } finally {
        if (!cancelled && showSpinner) setIsRefreshingWallets(false);
      }
    };

    void refresh(true);

    const intervalId = window.setInterval(() => {
      void refresh(false);
    }, 1500);

    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void refresh(false);
      }
    };
    const onFocus = () => {
      void refresh(false);
    };

    document.addEventListener('visibilitychange', onVisible);
    window.addEventListener('focus', onFocus);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
      document.removeEventListener('visibilitychange', onVisible);
      window.removeEventListener('focus', onFocus);
    };
  }, [open, step, onRefreshWallets]);

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

  const handleInstallClick = (
    event: React.MouseEvent,
    walletId: SupportedWallet
  ) => {
    event.stopPropagation();
    window.open(
      WALLET_INSTALL_URLS[walletId] ?? 'https://www.freighter.app/',
      '_blank',
      'noopener,noreferrer'
    );
  };

  const handleWalletSelect = async (wallet: AvailableWallet) => {
    dismissedRef.current = false;
    const attempt = ++connectAttemptRef.current;

    // Always attempt connect — detection can lag behind a freshly installed extension.
    if (onRefreshWallets) {
      setIsRefreshingWallets(true);
      try {
        await onRefreshWallets();
      } finally {
        setIsRefreshingWallets(false);
      }
    }

    if (dismissedRef.current || connectAttemptRef.current !== attempt) {
      return;
    }

    onNetworkSelection?.(selectedNetwork);
    setSelectedWallet(wallet.id);
    setConnectionError(null);
    setStep('connecting');

    try {
      await onConnect(wallet.id);
      // Success UI is advanced via walletNetwork effect once the provider updates.
    } catch (err) {
      if (dismissedRef.current || connectAttemptRef.current !== attempt) {
        return;
      }
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

  const dismissModal = () => {
    dismissedRef.current = true;
    connectAttemptRef.current += 1;
    onOpenChange(false);
    resetFlow();
  };

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      dismissModal();
    }
  };

  const handleCancelConnecting = () => {
    dismissedRef.current = true;
    connectAttemptRef.current += 1;
    setStep('select-wallet');
    setConnectionError(null);
  };

  const handleManualRefresh = async () => {
    if (!onRefreshWallets) return;
    setIsRefreshingWallets(true);
    try {
      await onRefreshWallets();
    } finally {
      setIsRefreshingWallets(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="wallet-connect-dialog"
        className="flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-4 sm:p-6 sm:max-w-[425px] md:max-w-[600px] pb-[max(1rem,env(safe-area-inset-bottom))]"
      >
        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
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
                      <strong>Freighter</strong> — browser extension (recommended for testnet)
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-primary mt-0.5">✓</span>
                    <span>
                      <strong>xBull</strong> — browser extension / web wallet
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-primary mt-0.5">✓</span>
                    <span>
                      <strong>Albedo</strong> — hosted web wallet (no extension required)
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-primary mt-0.5">✓</span>
                    <span>
                      <strong>LOBSTR</strong> — browser signer extension
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
                <Button variant="outline" onClick={dismissModal} className="flex-1">
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
                      disabled={isLoading || isRefreshingWallets}
                      className={`relative p-4 rounded-lg border-2 transition-all text-left ${
                        !wallet.installed
                          ? 'border-dashed border-muted-foreground/50 bg-muted/30 hover:border-primary hover:bg-muted/50'
                          : 'border-border hover:border-primary hover:bg-accent'
                      } disabled:opacity-50`}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <div>
                          <h4 className="font-semibold">{wallet.label}</h4>
                          <p className="text-sm text-muted-foreground">
                            {wallet.installed
                              ? 'Detected — click to connect'
                              : 'Not detected yet — click to try connecting'}
                          </p>
                          {wallet.id === 'xbull' && (
                            <p className="text-xs text-muted-foreground mt-1">
                              Transaction signing is supported on testnet only.
                            </p>
                          )}
                        </div>
                        {!wallet.installed && wallet.id !== 'albedo' && (
                          <span
                            role="link"
                            tabIndex={0}
                            onClick={(event) => handleInstallClick(event, wallet.id)}
                            onKeyDown={(event) => {
                              if (event.key === 'Enter' || event.key === ' ') {
                                handleInstallClick(
                                  event as unknown as React.MouseEvent,
                                  wallet.id
                                );
                              }
                            }}
                            className="inline-flex items-center gap-1 shrink-0 rounded-md border px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:border-primary"
                            aria-label={`Install ${wallet.label}`}
                          >
                            Install
                            <ExternalLink className="h-3.5 w-3.5" />
                          </span>
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

              <p className="text-xs text-muted-foreground">
                Just installed an extension? Use refresh, or wait a moment — we keep checking
                while this dialog is open.
              </p>

              <div className="flex gap-2 pt-2">
                <Button
                  variant="outline"
                  onClick={() =>
                    setStep(allowedNetworks.length > 1 ? 'select-network' : 'welcome')
                  }
                  className="flex-1"
                >
                  Back
                </Button>
                {onRefreshWallets && (
                  <Button
                    variant="outline"
                    onClick={() => void handleManualRefresh()}
                    disabled={isRefreshingWallets}
                    className="flex-1"
                  >
                    {isRefreshingWallets ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <RefreshCw className="h-4 w-4" />
                    )}
                    <span className="ml-2">Refresh</span>
                  </Button>
                )}
              </div>
            </div>
          </>
        )}

        {step === 'connecting' && (
          <>
            <DialogHeader>
              <DialogTitle>
                Connecting{' '}
                {selectedWallet ? WALLET_LABELS[selectedWallet] : 'wallet'}
              </DialogTitle>
              <DialogDescription>
                Please approve the connection in your wallet
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-6 py-8 flex flex-col items-center">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <div className="text-center space-y-2">
                <p className="font-medium">Waiting for approval...</p>
                <p className="text-sm text-muted-foreground max-w-sm">
                  Check the Freighter extension popup (puzzle icon in your browser
                  toolbar). If nothing appears, unlock Freighter and try again.
                </p>
              </div>
              <div className="flex w-full gap-2">
                <Button
                  variant="outline"
                  onClick={handleCancelConnecting}
                  className="flex-1"
                >
                  Cancel
                </Button>
                <Button
                  variant="outline"
                  onClick={dismissModal}
                  className="flex-1"
                  data-testid="wallet-connect-dismiss"
                >
                  Close dialog
                </Button>
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
              <Button onClick={dismissModal} className="w-full">
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
                  <li>Ensure your wallet extension/app is enabled and unlocked</li>
                  <li>Use Refresh on the wallet list after installing an extension</li>
                  <li>Try refreshing the page</li>
                  <li>Check that you&apos;re using the correct network</li>
                </ul>
              </div>

              {selectedWallet && selectedWallet !== 'albedo' && (
                <Button
                  variant="outline"
                  className="w-full"
                  onClick={(event) => handleInstallClick(event, selectedWallet)}
                >
                  Install {WALLET_LABELS[selectedWallet]}
                  <ExternalLink className="ml-2 h-4 w-4" />
                </Button>
              )}

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
                <Button onClick={dismissModal} className="flex-1">
                  Close
                </Button>
              </div>
            </div>
          </>
        )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
