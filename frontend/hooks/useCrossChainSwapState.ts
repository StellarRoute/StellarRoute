'use client';

import { useCallback, useMemo, useState } from 'react';
import {
  CHAIN_DEFINITIONS,
  findCorridorById,
  findCorridorForChains,
  isCorridorExecutable,
  resolveCorridorAvailability,
  UNMATCHED_CORRIDOR_ID,
  chainFamilyForDisplayId,
} from '@/lib/cross-chain/corridors';
import { buildExecutionTimelineSteps } from '@/lib/cross-chain/execution-timeline';
import { validatePreviewRecipientAddress } from '@/lib/cross-chain/recipient-validation';
import { resolveCctpDirection } from '@/lib/cctp/corridor-bridge';
import type {
  ChainDisplayId,
  CorridorId,
  CorridorSelectionId,
  ExecutionTimelineStep,
} from '@/lib/cross-chain/types';
import type { CctpSessionRecoveryMeta } from '@/lib/cctp/session-vault';

export function useCrossChainSwapState(options?: {
  timelineStepsOverride?: ExecutionTimelineStep[];
  initialSourceChainId?: ChainDisplayId;
  initialDestChainId?: ChainDisplayId;
}) {
  const [sourceChainId, setSourceChainId] = useState<ChainDisplayId>(
    options?.initialSourceChainId ?? 'stellar'
  );
  const [destChainId, setDestChainId] = useState<ChainDisplayId>(
    options?.initialDestChainId ?? 'ethereum-sepolia'
  );
  const [sourceAmount, setSourceAmount] = useState('');
  const [recipientOverride, setRecipientOverride] = useState('');
  const [useRecipientOverride, setUseRecipientOverride] = useState(false);

  const catalogMatch = useMemo(
    () => findCorridorForChains(sourceChainId, destChainId),
    [sourceChainId, destChainId]
  );

  const isUncatalogued = catalogMatch === null;
  const corridor = catalogMatch;
  const corridorId: CorridorSelectionId = isUncatalogued
    ? UNMATCHED_CORRIDOR_ID
    : corridor!.id;

  const availability = isUncatalogued
    ? 'unsupported'
    : resolveCorridorAvailability(corridor!);

  // Catalog/backend-route executable. CCTP settlement still gates on /api/v2 readiness.
  const executable = !isUncatalogued && isCorridorExecutable(corridor!);
  const isStellarNativePair =
    sourceChainId === 'stellar' && destChainId === 'stellar';
  const isStellarNativeExecutable = executable && isStellarNativePair;

  const destChainFamily = chainFamilyForDisplayId(destChainId);
  const recipientValidation = useMemo(() => {
    if (!useRecipientOverride || !recipientOverride.trim()) {
      return { valid: true as const };
    }
    return validatePreviewRecipientAddress(destChainFamily, recipientOverride);
  }, [useRecipientOverride, recipientOverride, destChainFamily]);

  const canReview =
    executable && !isStellarNativeExecutable && recipientValidation.valid;

  const selectCorridor = useCallback((id: CorridorId) => {
    const fromCatalog = findCorridorById(id);
    setSourceChainId(fromCatalog.sourceChainId);
    setDestChainId(fromCatalog.destChainId);
  }, []);

  const selectSourceChain = useCallback((id: ChainDisplayId) => {
    setSourceChainId(id);
  }, []);

  const selectDestChain = useCallback((id: ChainDisplayId) => {
    setDestChainId(id);
  }, []);

  const restoreFromRecovery = useCallback((recovery: CctpSessionRecoveryMeta) => {
    setSourceChainId(recovery.sourceChainId as ChainDisplayId);
    setDestChainId(recovery.destChainId as ChainDisplayId);
    setSourceAmount(recovery.amount);
    // Recipient comes from the connected destination wallet — no paste override.
    setRecipientOverride('');
    setUseRecipientOverride(false);
  }, []);

  const isCctpCorridor =
    !isUncatalogued &&
    Boolean(resolveCctpDirection(sourceChainId, destChainId));

  const timelineSteps = useMemo(() => {
    if (options?.timelineStepsOverride) {
      return options.timelineStepsOverride;
    }
    if (executable && isCctpCorridor) {
      return buildExecutionTimelineSteps(true, false).map((step) =>
        step.id === 'stellar_swap'
          ? { ...step, status: 'unavailable' as const }
          : step.id === 'burn'
            ? { ...step, status: 'pending' as const }
            : step,
      );
    }
    return buildExecutionTimelineSteps(executable, isStellarNativeExecutable);
  }, [
    options?.timelineStepsOverride,
    executable,
    isStellarNativeExecutable,
    isCctpCorridor,
  ]);

  return {
    corridorId,
    corridor,
    isUncatalogued,
    availability,
    executable,
    isCctpCorridor,
    isStellarNativePair,
    isStellarNativeExecutable,
    sourceChainId,
    destChainId,
    sourceChain: CHAIN_DEFINITIONS[sourceChainId],
    destChain: CHAIN_DEFINITIONS[destChainId],
    sourceAmount,
    setSourceAmount,
    recipientOverride,
    setRecipientOverride,
    useRecipientOverride,
    setUseRecipientOverride,
    recipientValidation,
    canReview,
    selectCorridor,
    selectSourceChain,
    selectDestChain,
    restoreFromRecovery,
    timelineSteps,
  };
}
