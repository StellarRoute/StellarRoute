'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import dynamic from 'next/dynamic';
import { toast } from 'sonner';
import { NetworkMismatchBanner } from '@/components/shared/NetworkMismatchBanner';
import { Button } from '@/components/ui/button';
import { useFeatureFlag } from '@/hooks/useFeatureFlag';
import { useCrossChainSwapState } from '@/hooks/useCrossChainSwapState';
import { useApiV2Readiness } from '@/hooks/useApiV2Readiness';
import { useCctpSaga } from '@/hooks/useCctpSaga';
import { useCrossChainWalletRoles } from '@/hooks/useCrossChainWalletRoles';
import { UNMATCHED_CORRIDOR_ID } from '@/lib/cross-chain/corridors';
import { resolveCctpDirection } from '@/lib/cctp/corridor-bridge';
import { loadCctpSession } from '@/lib/cctp/session-vault';
import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { CctpExecutionPanel } from './CctpExecutionPanel';
import { CctpCompleteDialog } from './CctpCompleteDialog';
import { CorridorTabs } from './CorridorTabs';
import { PairedChainSelectors } from './PairedChainSelectors';
import { UnsupportedCorridorState } from './UnsupportedCorridorState';
import type { CrossChainDeckStoryPresentation } from './crossChainStoryPresentation';
import {
  isCctpPrimaryActionDisabled,
  resolveCctpCtaHint,
  resolveDestinationWalletSetupHint,
} from './cctpCtaHint';
import {
  cctpNeedsUserAction,
  resolveCctpNextActionNotice,
  resolveCctpNextActionToast,
} from './cctpNextActionNotice';

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
  const [completeOpen, setCompleteOpen] = useState(false);
  const completedTransferIdRef = useRef<string | null>(null);
  const prevUserActionRef = useRef<string | null>(null);

  const walletRoles = useCrossChainWalletRoles({
    sourceChainId: state.sourceChainId,
    destChainId: state.destChainId,
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
      state.sourceAmount === recovery.amount;
    if (formMatches || restoredRecoveryKeyRef.current === key) return;
    restoredRecoveryKeyRef.current = key;
    state.restoreFromRecovery(recovery);
  }, [
    saga.sessionPublic?.recovery,
    state.destChainId,
    state.sourceAmount,
    state.sourceChainId,
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
    setCompleteOpen(false);
    completedTransferIdRef.current = null;
  };

  const handleCompleteDone = () => {
    saga.resetSaga();
    setConfirmAbandon(false);
    setCompleteOpen(false);
    completedTransferIdRef.current = null;
  };

  useEffect(() => {
    const transferId = saga.transferStatus?.transfer_id;
    const isComplete =
      saga.stage === 'completed' || saga.transferStatus?.status === 'completed';
    if (!isComplete || !transferId) return;
    if (completedTransferIdRef.current === transferId) return;
    completedTransferIdRef.current = transferId;
    setCompleteOpen(true);
  }, [saga.stage, saga.transferStatus?.status, saga.transferStatus?.transfer_id]);

  const bridgeReady = readiness.cctpGloballyReady;
  const cctpBlockInput = useMemo(
    () => ({
      direction: walletRoles.direction,
      sourceAmount: state.sourceAmount,
      destRecipientAddress: walletRoles.destRecipientAddress,
      bridgeReady,
      readinessLoading: readiness.loading,
      sagaPrimaryDisabled: saga.primaryAction.disabled,
    }),
    [
      bridgeReady,
      readiness.loading,
      saga.primaryAction.disabled,
      state.sourceAmount,
      walletRoles.destRecipientAddress,
      walletRoles.direction,
    ],
  );
  const ctaHint = useMemo(
    () => resolveCctpCtaHint(cctpBlockInput),
    [cctpBlockInput],
  );
  const cctpPrimaryDisabled = useMemo(
    () => isCctpPrimaryActionDisabled(cctpBlockInput),
    [cctpBlockInput],
  );
  const destinationWalletSetupHint = useMemo(
    () =>
      resolveDestinationWalletSetupHint(
        walletRoles.direction,
        walletRoles.destRecipientAddress,
      ),
    [walletRoles.destRecipientAddress, walletRoles.direction],
  );

  const needsUserAction = cctpNeedsUserAction(
    saga.primaryAction.action,
    cctpPrimaryDisabled,
  );
  const nextActionNotice = needsUserAction
    ? resolveCctpNextActionNotice(saga.primaryAction.action)
    : null;

  useEffect(() => {
    const action = saga.primaryAction.action;
    if (!needsUserAction) {
      prevUserActionRef.current = action;
      return;
    }
    if (prevUserActionRef.current === action) return;
    prevUserActionRef.current = action;
    const toastCopy = resolveCctpNextActionToast(action);
    if (toastCopy) {
      toast.message(toastCopy, { id: 'cctp-next-action' });
    }
  }, [needsUserAction, saga.primaryAction.action]);

  return (
    <div
      className="cross-chain-deck w-full mx-auto space-y-6"
      data-testid="cross-chain-swap-deck"
    >
      <header className="space-y-1.5 px-1 sm:px-0">
        <div className="space-y-1">
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-primary">
            Cross-chain
          </p>
          <h2 className="brand-wordmark text-2xl text-foreground sm:text-3xl">
            Bridge USDC
          </h2>
        </div>
      </header>

      <CorridorTabs
        activeId={state.corridorId}
        onSelect={state.selectCorridor}
        disabled={saga.inputsLocked}
      />

      {!state.isStellarNativeExecutable && (
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
          destWalletHint={destinationWalletSetupHint}
        />
      )}

      <div
        className="cross-chain-deck-main space-y-4 min-w-0"
        id={panelId}
        role="tabpanel"
        aria-labelledby={panelLabelId}
      >
        {state.isStellarNativeExecutable ? (
          <div className="space-y-3" data-testid="stellar-native-delegation">
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
                {state.executable && walletRoles.direction && (
                  <label className="block space-y-1">
                    <span className="text-xs font-medium text-muted-foreground">
                      Amount
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
                    <p
                      className="text-xs text-muted-foreground"
                      data-testid="cctp-usdc-only-note"
                    >
                      Bridges USDC only.{' '}
                      <Button
                        type="button"
                        variant="link"
                        className="h-auto p-0 text-xs font-normal"
                        onClick={() => state.selectCorridor('stellar-native')}
                        data-testid="swap-to-usdc-on-stellar-link"
                      >
                        Swap to USDC on Stellar
                      </Button>
                    </p>
                  </label>
                )}
                {state.executable && walletRoles.direction && (
                  <CctpExecutionPanel
                    stage={saga.stage}
                    quote={saga.quote}
                    transferStatus={saga.transferStatus}
                    error={saga.error}
                    primaryLabel={saga.primaryAction.label}
                    primaryDisabled={cctpPrimaryDisabled}
                    needsUserAction={needsUserAction}
                    nextActionNotice={nextActionNotice}
                    ctaHint={ctaHint}
                    onPrimary={() => void saga.runPrimaryAction()}
                    onReset={handleAbandon}
                    onCompleteDone={handleCompleteDone}
                    onViewReceipt={() => setCompleteOpen(true)}
                    recipient={
                      saga.sessionPublic?.recovery.recipient ??
                      walletRoles.destRecipientAddress
                    }
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

      <CctpCompleteDialog
        open={completeOpen}
        onOpenChange={setCompleteOpen}
        quote={saga.quote}
        transferStatus={saga.transferStatus}
        recipient={
          saga.sessionPublic?.recovery.recipient ??
          walletRoles.destRecipientAddress
        }
        onDone={handleCompleteDone}
      />
    </div>
  );
}
