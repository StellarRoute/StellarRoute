'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTransactionLifecycle } from './useTransactionLifecycle';
import type { TradeParams } from './useTransactionLifecycle';
import type { TransactionStatus } from '@/types/transaction';
import type { PreSubmitSnapshot, RollbackTarget } from '@/types/transaction';
import type { NotificationPreference } from '@/lib/notifications';

export interface UseOptimisticSwapOptions {
  deadlineMs?: number;
  signTransaction?: (xdr: string) => Promise<string>;
  submitTransaction?: (signedXdr: string) => Promise<{ hash: string }>;
  rollbackTarget: RollbackTarget;
  /** Notification preference — passed through to useTransactionLifecycle */
  notificationPreference?: NotificationPreference;
}

export interface UseOptimisticSwapResult {
  status: TransactionStatus | 'review';
  txHash: string | undefined;
  errorMessage: string | undefined;
  tradeParams: TradeParams | undefined;
  submitLock: boolean;
  snapshot: PreSubmitSnapshot | null;
  initiateSwap: (params: TradeParams & { snapshot: PreSubmitSnapshot }) => void;
  cancel: () => void;
  resubmit: () => Promise<void>;
  tryAgain: () => void;
  dismiss: () => void;
}

export function useOptimisticSwap(options: UseOptimisticSwapOptions): UseOptimisticSwapResult {
  const { rollbackTarget, ...lifecycleOptions } = options;

  const lifecycle = useTransactionLifecycle(lifecycleOptions);
  const [submitLock, setSubmitLock] = useState(false);
  const [snapshot, setSnapshot] = useState<PreSubmitSnapshot | null>(null);
  // Synchronous guard for same-render-cycle duplicate calls
  const lockRef = useRef(false);
  // Keep rollbackTarget in a ref so the effect closure always has the latest version
  const rollbackTargetRef = useRef(rollbackTarget);
  const snapshotRef = useRef<PreSubmitSnapshot | null>(null);

  useEffect(() => {
    rollbackTargetRef.current = rollbackTarget;
  }, [rollbackTarget]);

  function applyRollback(snap: PreSubmitSnapshot) {
    const target = rollbackTargetRef.current;
    target.setFromToken(snap.fromToken);
    target.setToToken(snap.toToken);
    target.setFromAmount(snap.fromAmount);
    target.setSlippage(snap.slippage);
    target.setSelectedRoute(snap.selectedRouteId);
    target.refreshQuote();
  }

  // Watch for terminal states → release lock + rollback on failure
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => {
    if (lifecycle.status === 'confirmed') {
      setSubmitLock(false);
      lockRef.current = false;
    } else if (lifecycle.status === 'failed' || lifecycle.status === 'dropped') {
      const snap = snapshotRef.current;
      if (snap) {
        applyRollback(snap);
      } else {
        console.error('[useOptimisticSwap] Rollback triggered but PreSubmitSnapshot is absent');
      }
      setSubmitLock(false);
      lockRef.current = false;
    }
  }, [lifecycle.status]);

  const initiateSwap = useCallback(
    (params: TradeParams & { snapshot: PreSubmitSnapshot }) => {
      // Synchronous guard — rejects same-render-cycle duplicates before state update propagates
      if (lockRef.current) return;
      lockRef.current = true;
      setSubmitLock(true);
      const { snapshot: snap, ...tradeParams } = params;
      setSnapshot(snap);
      snapshotRef.current = snap;
      lifecycle.initiateSwap(tradeParams);
    },
    [lifecycle]
  );

  const cancel = useCallback(() => {
    lifecycle.cancel();
    const snap = snapshotRef.current;
    if (snap) {
      applyRollback(snap);
    }
    setSubmitLock(false);
    lockRef.current = false;
  }, [lifecycle]);

  return {
    status: lifecycle.status,
    txHash: lifecycle.txHash,
    errorMessage: lifecycle.errorMessage,
    tradeParams: lifecycle.tradeParams,
    submitLock,
    snapshot,
    initiateSwap,
    cancel,
    resubmit: lifecycle.resubmit,
    tryAgain: lifecycle.tryAgain,
    dismiss: lifecycle.dismiss,
  };
}
