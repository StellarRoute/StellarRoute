'use client';

import { CctpStepRail, cctpActiveStepFromSaga, cctpCompletedStepsFromSaga } from './CctpStepRail';
import { formatChainPairLabel } from '@/lib/cross-chain/format';
import type { ChainDisplayId, CrossChainProtocol } from '@/lib/cross-chain/types';
import type { CctpQuoteResponse } from '@/lib/cctp/types';
import { cn } from '@/lib/utils';
import { ArrowRight } from 'lucide-react';

interface CrossChainRoutePanelProps {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  protocol: CrossChainProtocol | null;
  executable: boolean;
  uncatalogued?: boolean;
  quote?: CctpQuoteResponse | null;
  bridgeUnavailable?: boolean;
  sagaStatus?: string;
  className?: string;
}

export function CrossChainRoutePanel({
  sourceChainId,
  destChainId,
  protocol,
  executable,
  uncatalogued = false,
  quote = null,
  bridgeUnavailable = false,
  sagaStatus,
  className,
}: CrossChainRoutePanelProps) {
  const pairLabel = formatChainPairLabel(sourceChainId, destChainId);

  if (uncatalogued) {
    return (
      <section
        aria-label="Route preview"
        className={cn(
          'space-y-3 rounded-2xl border border-border/40 bg-card/50 p-4 sm:p-5',
          className
        )}
        data-testid="cross-chain-route-panel"
      >
        <div className="space-y-1">
          <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
            Route rail
          </p>
          <h2 className="brand-wordmark text-lg text-foreground">{pairLabel}</h2>
        </div>
        <p className="text-sm text-muted-foreground" role="status">
          This chain pair is not in the corridor catalog. No protocol rail, burn,
          attest, or mint preview is shown until a catalog corridor exists.
        </p>
      </section>
    );
  }

  const isCctp = protocol === 'cctp-preview';

  return (
    <section
      aria-label="Route preview"
      className={cn(
        'space-y-4 rounded-2xl border border-border/40 bg-card/50 p-4 sm:p-5',
        className
      )}
      data-testid="cross-chain-route-panel"
    >
      <div className="space-y-1">
        <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
          Route rail
        </p>
        <h2 className="brand-wordmark text-lg text-foreground">{pairLabel}</h2>
        <p className="text-sm text-muted-foreground">
          {protocol === 'stellar-native'
            ? 'Same-chain Stellar swap — SDEX and Soroban venues via the existing swap path.'
            : executable
              ? 'Circle CCTP — server-prepared payloads, wallet sign, hash-only submit.'
              : bridgeUnavailable
                ? 'CCTP corridor is listed but not executable on this API yet.'
                : 'Protocol preview — quotes and execution are not available for this corridor yet.'}
        </p>
      </div>

      <div className="flex items-center justify-center gap-3 py-2">
        <HubNode
          label={sourceChainId === 'stellar' ? 'Stellar' : 'Source'}
          active
        />
        <ArrowRight className="h-5 w-5 text-primary shrink-0" aria-hidden />
        <HubNode
          label={
            protocol === 'stellar-native'
              ? 'SDEX / Soroban'
              : protocol === 'cctp-preview'
                ? 'CCTP'
                : 'Route'
          }
          active
          emphasized
        />
        <ArrowRight className="h-5 w-5 text-primary shrink-0" aria-hidden />
        <HubNode
          label={
            destChainId === 'stellar'
              ? 'Stellar'
              : destChainId === 'ethereum-sepolia'
                ? 'ETH Sepolia'
                : 'Destination'
          }
          active
        />
      </div>

      {isCctp && (
        <div className="space-y-2" data-testid="cctp-route-rail">
          <p className="text-xs font-semibold text-foreground">CCTP rail</p>
          <CctpStepRail
            previewOnly={!executable}
            activeStep={executable ? cctpActiveStepFromSaga(sagaStatus) : null}
            completedSteps={
              executable ? cctpCompletedStepsFromSaga(sagaStatus) : []
            }
          />
          <dl className="grid gap-2 text-xs sm:grid-cols-2">
            <div className="rounded-lg border border-border/30 bg-muted/20 p-2">
              <dt className="text-muted-foreground">Provider</dt>
              <dd className="font-medium">Circle CCTP</dd>
            </div>
            <div className="rounded-lg border border-border/30 bg-muted/20 p-2">
              <dt className="text-muted-foreground">Fees &amp; finality</dt>
              <dd className="font-medium text-muted-foreground">
                {quote
                  ? `${quote.finality === 'fast' ? 'Fast' : 'Standard'} · quote until ${new Date(quote.expires_at * 1000).toLocaleTimeString()}`
                  : executable
                    ? 'Request a quote for fee estimate'
                    : 'Estimate unavailable — corridor not live'}
              </dd>
            </div>
          </dl>
        </div>
      )}

      {protocol === 'stellar-native' && (
        <p className="text-xs text-muted-foreground">
          Same-chain routing aggregates SDEX and Soroban venues via the existing
          Stellar swap path.
        </p>
      )}
    </section>
  );
}

function HubNode({
  label,
  active,
  emphasized = false,
}: {
  label: string;
  active: boolean;
  emphasized?: boolean;
}) {
  return (
    <div
      className={cn(
        'flex min-h-11 min-w-[88px] flex-col items-center justify-center rounded-xl border px-3 py-2 text-center',
        active
          ? emphasized
            ? 'border-primary/50 bg-primary/15 text-foreground'
            : 'border-primary/35 bg-primary/8'
          : 'border-border/40 bg-background/40 text-muted-foreground'
      )}
    >
      <span className="text-[10px] uppercase tracking-wide">Hub</span>
      <span className="text-xs font-semibold">{label}</span>
    </div>
  );
}
