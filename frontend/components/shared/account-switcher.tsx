"use client";

import * as React from "react";
import { useWallet } from "@/components/providers/wallet-provider";
import { checkAddressChange } from "@/lib/wallet";
import type { AccountSwitchState } from "@/lib/wallet/types";

interface AccountSwitcherProps {
  onAccountChange?: (newAddress: string) => void;
}

/**
 * Only surfaces UI when something needs attention (account change / txn lock).
 * Idle refresh lives in the wallet dropdown to keep the header uncluttered.
 */
export function AccountSwitcher({ onAccountChange }: AccountSwitcherProps) {
  const {
    address,
    walletId,
    isConnected,
    refreshAccount,
    isTransactionPending,
  } = useWallet();

  const [localSwitchState, setLocalSwitchState] = React.useState<AccountSwitchState>({
    isDetecting: false,
    hasChanged: false,
    previousAddress: null,
  });
  const [isRefreshing, setIsRefreshing] = React.useState(false);

  React.useEffect(() => {
    if (!isConnected || !walletId || !address || isTransactionPending) return;

    const checkForChanges = async () => {
      setLocalSwitchState((prev) => ({ ...prev, isDetecting: true }));

      try {
        const newAddress = await checkAddressChange(walletId, address);
        if (newAddress && newAddress !== address) {
          setLocalSwitchState({
            isDetecting: false,
            hasChanged: true,
            previousAddress: address,
          });
        } else {
          setLocalSwitchState((prev) => ({ ...prev, isDetecting: false }));
        }
      } catch {
        setLocalSwitchState((prev) => ({ ...prev, isDetecting: false }));
      }
    };

    const interval = setInterval(checkForChanges, 3000);
    return () => clearInterval(interval);
  }, [isConnected, walletId, address, isTransactionPending]);

  const handleRefreshAccount = async () => {
    if (!walletId || isTransactionPending) return;

    setIsRefreshing(true);
    try {
      const previousAddress = address;
      await refreshAccount();

      setLocalSwitchState({
        isDetecting: false,
        hasChanged: false,
        previousAddress: null,
      });

      if (previousAddress && address && previousAddress !== address && onAccountChange) {
        onAccountChange(address);
      }
    } catch (error) {
      console.error("Failed to refresh account:", error);
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleDismissChange = () => {
    setLocalSwitchState({
      isDetecting: false,
      hasChanged: false,
      previousAddress: null,
    });
  };

  if (!isConnected) return null;

  if (isTransactionPending) {
    return (
      <div className="hidden rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive lg:block">
        Account switching paused while a transaction is pending
      </div>
    );
  }

  if (localSwitchState.hasChanged) {
    return (
      <div className="absolute top-full right-0 z-50 mt-2 w-72 rounded-xl border border-signal/35 bg-card p-3 shadow-lg">
        <p className="text-sm font-semibold text-foreground">Account changed</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Your wallet switched accounts. Refresh to use the new one.
        </p>
        {localSwitchState.previousAddress && (
          <p className="mt-2 font-mono text-[11px] text-muted-foreground">
            Previous: {localSwitchState.previousAddress.slice(0, 6)}…
            {localSwitchState.previousAddress.slice(-6)}
          </p>
        )}
        <div className="mt-3 flex gap-2">
          <button
            type="button"
            onClick={handleRefreshAccount}
            disabled={isRefreshing}
            className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isRefreshing ? "Refreshing…" : "Refresh account"}
          </button>
          <button
            type="button"
            onClick={handleDismissChange}
            className="rounded-md border border-border px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            Dismiss
          </button>
        </div>
      </div>
    );
  }

  // Keep detection running silently — no idle chrome in the header.
  return null;
}
