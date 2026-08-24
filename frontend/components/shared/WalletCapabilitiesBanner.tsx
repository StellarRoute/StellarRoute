'use client';

import * as React from 'react';
import { useWallet } from '@/components/providers/wallet-provider';

export function WalletCapabilitiesBanner() {
  const { isConnected, capabilities } = useWallet();

  if (!isConnected || !capabilities || capabilities.missingCapabilities.length === 0) {
    return null;
  }

  return (
    <div
      role="alert"
      data-testid="wallet-capabilities-banner"
      className="w-full bg-amber-500/10 border-b border-amber-500/30 px-4 py-2 text-sm text-amber-200"
    >
      <div className="flex flex-col gap-1 max-w-7xl mx-auto">
        {capabilities.missingCapabilities.map((msg, index) => (
          <p key={index} data-testid="capability-warning">
            ⚠️ {msg}
          </p>
        ))}
      </div>
    </div>
  );
}
