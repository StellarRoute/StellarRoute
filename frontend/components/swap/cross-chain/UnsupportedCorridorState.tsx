'use client';

import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { formatChainPairLabel } from '@/lib/cross-chain/format';

interface UnsupportedCorridorStateProps {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  uncatalogued?: boolean;
  reason?: string;
}

export function UnsupportedCorridorState({
  sourceChainId,
  destChainId,
  uncatalogued = false,
  reason,
}: UnsupportedCorridorStateProps) {
  const pairLabel = formatChainPairLabel(sourceChainId, destChainId);

  const defaultReason = uncatalogued
    ? `${pairLabel} is not in the corridor catalog. No quote, destination amount, or execution is available for this pair.`
    : `${pairLabel} is visible in the catalog but not executable yet. No quote or destination amount is shown to avoid misleading estimates.`;

  return (
    <div
      role="alert"
      className="rounded-2xl border border-signal/35 bg-signal/8 p-4 space-y-2"
      data-testid="unsupported-corridor-alert"
    >
      <p className="text-sm font-semibold text-foreground">
        {uncatalogued ? `${pairLabel} — unsupported pair` : `${pairLabel} — coming soon`}
      </p>
      <p className="text-sm text-muted-foreground">{reason ?? defaultReason}</p>
      <p className="text-xs text-muted-foreground">
        Connect wallets to preview signing readiness, or switch to Stellar native
        for live swaps today.
      </p>
    </div>
  );
}
