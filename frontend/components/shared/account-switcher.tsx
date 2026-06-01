"use client";

import * as React from "react";
import { useWallet } from "@/components/providers/wallet-provider";
import { checkAddressChange } from "@/lib/wallet";
import type { AccountSwitchState } from "@/lib/wallet/types";

interface AccountSwitcherProps {
  onAccountChange?: (newAddress: string) => void;
}

export function AccountSwitcher({ onAccountChange }: AccountSwitcherProps) {
  const { 
    address, 
    walletId, 
    isConnected, 
    refreshAccount, 
    isTransactionPending,
    accountSwitchState 
  } = useWallet();
  
  const [localSwitchState, setLocalSwitchState] = React.useState<AccountSwitchState>({
    isDetecting: false,
    hasChanged: false,
    previousAddress: null,
  });
  const [isRefreshing, setIsRefreshing] = React.useState(false);

  // Monitor for account changes
  React.useEffect(() => {
    if (!isConnected || !walletId || !address || isTransactionPending) return;

    const checkForChanges = async () => {
      setLocalSwitchState(prev => ({ ...prev, isDetecting: true }));
      
      try {
        const newAddress = await checkAddressChange(walletId, address);
        if (newAddress && newAddress !== address) {
          setLocalSwitchState({
            isDetecting: false,
            hasChanged: true,
            previousAddress: address,
          });
        } else {
          setLocalSwitchState(prev => ({ ...prev, isDetecting: false }));
        }
      } catch {
        setLocalSwitchState(prev => ({ ...prev, isDetecting: false }));
      }
    };

    // Check every 3 seconds for account changes
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

      // Call the callback if address actually changed
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

  // Show transaction warning if trying to switch during transaction
  if (isTransactionPending) {
    return (
      <div className="rounded-md border border-red-200 bg-red-50 p-2">
        <div className="flex items-center gap-2">
          <span className="text-sm text-red-700">
            Account switching disabled during transaction
          </span>
        </div>
      </div>
    );
  }

  if (localSwitchState.hasChanged) {
    return (
      <div className="rounded-md border border-yellow-200 bg-yellow-50 p-3">
        <div className="flex items-start gap-3">
          <div className="flex-1">
            <h4 className="text-sm font-medium text-yellow-800">
              Account Change Detected
            </h4>
            <p className="mt-1 text-sm text-yellow-700">
              Your wallet account appears to have changed. Refresh to use the new account.
            </p>
            {localSwitchState.previousAddress && (
              <p className="mt-1 text-xs text-yellow-600">
                Previous: {localSwitchState.previousAddress.slice(0, 8)}...{localSwitchState.previousAddress.slice(-8)}
              </p>
            )}
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleRefreshAccount}
              disabled={isRefreshing}
              className="rounded-md bg-yellow-600 px-3 py-1 text-xs font-medium text-white hover:bg-yellow-700 disabled:opacity-50"
            >
              {isRefreshing ? "Refreshing..." : "Refresh Account"}
            </button>
            <button
              onClick={handleDismissChange}
              className="rounded-md border border-yellow-300 px-3 py-1 text-xs font-medium text-yellow-700 hover:bg-yellow-100"
            >
              Dismiss
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (localSwitchState.isDetecting) {
    return (
      <div className="rounded-md border border-blue-200 bg-blue-50 p-2">
        <div className="flex items-center gap-2">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"></div>
          <span className="text-sm text-blue-700">Checking for account changes...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={handleRefreshAccount}
        disabled={isRefreshing}
        className="rounded-md border px-2 py-1 text-xs hover:bg-gray-50 disabled:opacity-50"
        title="Refresh to check for account changes"
      >
        {isRefreshing ? "Refreshing..." : "↻ Refresh Account"}
      </button>
    </div>
  );
}
