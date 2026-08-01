'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import dynamic from 'next/dynamic';
import { NetworkMismatchBanner } from '@/components/shared/NetworkMismatchBanner';
import { useFeatureFlag } from '@/hooks/useFeatureFlag';
import { useCrossChainSwapState } from '@/hooks/useCrossChainSwapState';
import { useApiV2Readiness } from '@/hooks/useApiV2Readiness';
import { useCctpSaga } from '@/hooks/useCctpSaga';
import { useCrossChainWalletRoles } from '@/hooks/useCrossChainWalletRoles';
import { UNMATCHED_CORRIDOR_ID } from '@/lib/cross-chain/corridors';
import { resolveCctpDirection } from '@/lib/cctp/corridor-bridge';
import { loadCctpSession } from '@/lib/cctp/session-vault';
import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { corridorStatusCopy } from '@/lib/cross-chain/format';
import { cn } from '@/lib/utils';
import { CctpExecutionPanel } from './CctpExecutionPanel';
import { CorridorTabs } from './CorridorTabs';
import { CrossChainExecutionTimeline } from './CrossChainExecutionTimeline';
import { CrossChainRoutePanel } from './CrossChainRoutePanel';
import { DestinationAddressField } from './DestinationAddressField';
import { PairedChainSelectors } from './PairedChainSelectors';
import { RouteDisclosurePanel } from './RouteDisclosurePanel';
import { UnsupportedCorridorState } from './UnsupportedCorridorState';
import type { CrossChainDeckStoryPresentation } from './crossChainStoryPresentation';

const SwapCard = dynamic(
  () => import('@/components/swap/SwapCard').then((m) => m.SwapCard),
  {
    ssr: false,
    loading: () => (
      <div
        className="flex h-[480px] items-center justify-center rounded-2xl chart-panel"
        data-testid="swap-card-loading"
      >
        <div className="h-8 w-8 rounded-full border-4 border-primary border-t-transparent animate-spin" />
      </div>
    ),
  },
);

export interface CrossChainSwapDeckProps {
  storyPresentation?: CrossChainDeckStoryPresentation;
}

export function CrossChainSwapDeck({
  storyPresentation,
}: CrossChainSwapDeckProps = {}) {
  const state = useCrossChainSwapState({
    timelineStepsOverride: storyPresentation?.timelineSteps,
    initialSourceChainId: storyPresentation?.initialSourceChainId,
    initialDestChainId: storyPresentation?.initialDestChainId,
  });
  const { enabled: routesBeta } = useFeatureFlag('routes_beta');
  const readiness = useApiV2Readiness({ refreshMs: 60_000 });
  const [confirmAbandon, setConfirmAbandon] = useState(false);

  const walletRoles = useCrossChainWalletRoles({
    sourceChainId: state.sourceChainId,
    destChainId: state.destChainId,
    recipientOverride: state.recipientOverride,
    useRecipientOverride: state.useRecipientOverride,
  });

  const quoteInputsKey = useMemo(
    () =>
      [
        state.sourceChainId,
        state.destChainId,
        state.sourceAmount,
        walletRoles.destRecipientAddress,
        walletRoles.sagaWallets.sourceAddress ?? '',
        walletRoles.sagaWallets.mintSubmitter ?? '',
      ].join('|'),
    [
      state.sourceChainId,
      state.destChainId,
      state.sourceAmount,
      walletRoles.destRecipientAddress,
      walletRoles.sagaWallets,
    ],
  );

  const vaultResumeReady = useMemo(() => {
    const loaded = loadCctpSession();
    if (!loaded.ok) return false;
    const { sourceChainId, destChainId } = loaded.record.recovery;
    return Boolean(
      resolveCctpDirection(
        sourceChainId as ChainDisplayId,
        destChainId as ChainDisplayId,
      ),
    );
  }, [quoteInputsKey, state.sourceChainId, state.destChainId]);

  const saga = useCctpSaga({
    sourceChainId: state.sourceChainId,
    destChainId: state.destChainId,
    amount: state.sourceAmount || '0',
    recipient: walletRoles.destRecipientAddress,
    sender: walletRoles.sagaWallets.sourceAddress,
    mintSubmitter: walletRoles.sagaWallets.mintSubmitter,
    wallets: walletRoles.sagaWallets,
    bridgeReady:
      readiness.cctpGloballyReady &&
      ((state.executable && Boolean(walletRoles.direction)) || vaultResumeReady),
    quoteInputsKey,
  });

  const restoredRecoveryKeyRef = useRef<string | null>(null);
  useEffect(() => {
    const recovery = saga.sessionPublic?.recovery;
    if (!recovery?.sourceChainId || !recovery.destChainId) return;
    const key = `${recovery.sourceChainId}|${recovery.destChainId}|${recovery.amount}|${recovery.recipient}`;
    const formMatches =
      state.sourceChainId === recovery.sourceChainId &&
      state.destChainId === recovery.destChainId &&
      state.sourceAmount === recovery.amount &&
      state.useRecipientOverride &&
      state.recipientOverride === recovery.recipient;
    if (formMatches || restoredRecoveryKeyRef.current === key) return;
    restoredRecoveryKeyRef.current = key;
    state.restoreFromRecovery(recovery);
  }, [
    saga.sessionPublic?.recovery,
    state.destChainId,
    state.recipientOverride,
    state.sourceAmount,
    state.sourceChainId,
    state.useRecipientOverride,
    state.restoreFromRecovery,
  ]);

  const panelId =
    state.corridorId === UNMATCHED_CORRIDOR_ID
      ? 'corridor-panel-unmatched'
      : `corridor-panel-${state.corridorId}`;
  const panelLabelId =
    state.corridorId === UNMATCHED_CORRIDOR_ID
      ? 'corridor-tab-unmatched'
      : `corridor-tab-${state.corridorId}`;

  const showCrossChainPreview =
    !state.isStellarNativeExecutable && !state.isUncatalogued;
  const showUnsupported =
    state.isUncatalogued || (!state.executable && !state.isStellarNativeExecutable);

  const handleAbandon = () => {
    if (!confirmAbandon) {
      setConfirmAbandon(true);
      return;
    }
    saga.resetSaga();
    setConfirmAbandon(false);
  };

  return (
    <div
      className="cross-chain-deck w-full mx-auto space-y-5"
      data-testid="cross-chain-swap-deck"
    >
      <header className="space-y-2 px-1 sm:px-0">
        <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-primary">
          Cross-chain route
        </p>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <h2 className="brand-wordmark text-2xl text-foreground sm:text-3xl">
            Stellar-centered routing
          </h2>
          <span
            className={cn(
              'rounded-full border px-3 py-1 font-mono text-[10px] uppercase tracking-wider',
              state.executable && !state.isUncatalogued
                ? 'border-primary/40 bg-primary/10 text-primary'
                : 'border-border/50 text-muted-foreground',
            )}
            data-testid="corridor-status-badge"
          >
            {corridorStatusCopy(state.executable, state.isUncatalogued)}
          </span>
        </div>
        <p className="max-w-2xl text-sm text-muted-foreground">
          Source and destination chains stay visible. Only executable corridors
          reach review — previews explain protocol steps without fake quotes.
        </p>
      </header>

      <CorridorTabs
        activeId={state.corridorId}
        onSelect={state.selectCorridor}
        disabled={saga.inputsLocked}
      />

      <PairedChainSelectors
        sourceChainId={state.sourceChainId}
        destChainId={state.destChainId}
        onSourceChange={state.selectSourceChain}
        onDestChange={state.selectDestChain}
        sourceWalletState={storyPresentation?.sourceWalletState}
        destWalletState={storyPresentation?.destWalletState}
        sourceWalletBinding={walletRoles.sourceChipBinding}
        destWalletBinding={walletRoles.destChipBinding}
        mintSubmitterBinding={walletRoles.mintSubmitterChipBinding}
        inputsLocked={saga.inputsLocked}
      />

      <div
        className="cross-chain-deck-grid gap-5 lg:gap-6"
        id={panelId}
        role="tabpanel"
        aria-labelledby={panelLabelId}
      >
        <div className="space-y-4 min-w-0">
          {state.isStellarNativeExecutable ? (
            <div className="space-y-3" data-testid="stellar-native-delegation">
              <p className="text-xs text-muted-foreground">
                Amounts, assets, and quotes are edited in the Stellar swap card
                below — your single source for live execution.
              </p>
              <NetworkMismatchBanner />
              <SwapCard showRoutePicker={routesBeta} />
            </div>
          ) : (
            <div className="space-y-4">
              {showUnsupported && (
                <UnsupportedCorridorState
                  sourceChainId={state.sourceChainId}
                  destChainId={state.destChainId}
                  uncatalogued={state.isUncatalogued}
                />
              )}
              {showCrossChainPreview && (
                <>
                  <DestinationAddressField
                    chain={state.destChain}
                    enabled={state.useRecipientOverride}
                    onEnabledChange={state.setUseRecipientOverride}
                    value={state.recipientOverride}
                    onChange={state.setRecipientOverride}
                    validation={state.recipientValidation}
                    disabled={saga.inputsLocked}
                  />
                  {state.executable && walletRoles.direction && (
                    <label className="block space-y-1">
                      <span className="text-xs font-medium text-muted-foreground">
                        USDC amount (source)
                      </span>
                      <input
                        type="text"
                        inputMode="decimal"
                        className="min-h-11 w-full rounded-xl border border-border/50 bg-background/60 px-3 font-mono text-sm disabled:opacity-60"
                        value={state.sourceAmount}
                        onChange={(e) => state.setSourceAmount(e.target.value)}
                        placeholder="0.00"
                        data-testid="cctp-source-amount"
                        disabled={saga.inputsLocked}
                      />
                    </label>
                  )}
                  {state.executable && walletRoles.direction && (
                    <CctpExecutionPanel
                      stage={saga.stage}
                      quote={saga.quote}
                      transferStatus={saga.transferStatus}
                      error={saga.error}
                      primaryLabel={saga.primaryAction.label}
                      primaryDisabled={
                        saga.primaryAction.disabled ||
                        !state.sourceAmount ||
                        !walletRoles.destRecipientAddress ||
                        readiness.loading
                      }
                      onPrimary={() => void saga.runPrimaryAction()}
                      onReset={handleAbandon}
                      resetLabel={
                        confirmAbandon
                          ? 'Confirm abandon transfer'
                          : 'Start new transfer'
                      }
                      bridgeUnavailable={
                        readiness.loaded && !readiness.cctpGloballyReady
                      }
                      resumeMismatch={saga.resumeMismatch}
                      walletRoleMismatch={saga.walletRoleMismatch}
                      sessionPublic={saga.sessionPublic}
                      reattestCooldownUntil={saga.reattestCooldownUntil}
                    />
                  )}
                </>
              )}
            </div>
          )}
        </div>

        <aside className="space-y-4 min-w-0" aria-label="Route and execution details">
          <CrossChainRoutePanel
            sourceChainId={state.sourceChainId}
            destChainId={state.destChainId}
            protocol={state.corridor?.protocol ?? null}
            executable={
              state.isStellarNativeExecutable ||
              (state.executable && readiness.cctpGloballyReady)
            }
            uncatalogued={state.isUncatalogued}
            quote={saga.quote}
            bridgeUnavailable={
              !state.isStellarNativeExecutable &&
              readiness.loaded &&
              !readiness.cctpGloballyReady
            }
            sagaStatus={saga.transferStatus?.status}
          />
          <RouteDisclosurePanel />
          <CrossChainExecutionTimeline steps={state.timelineSteps} />
        </aside>
      </div>
    </div>
  );
}
