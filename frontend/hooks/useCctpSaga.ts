'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { StellarRouteApiError } from '@/lib/api/client';
import { buildCctpQuoteRequest } from '@/lib/cctp/corridor-bridge';
import { classifyStellarRecipient } from '@/lib/cctp/wallet-role-binding';
import { getCctpApiClient } from '@/lib/cctp/client';
import { mapCctpError, isStaleSequenceError, type CctpTraderError } from '@/lib/cctp/errors';
import { fingerprintPreparedPayload } from '@/lib/cctp/payload-fingerprint';
import {
  buildCctpSessionRecord,
  buildAutoReconcileRevision,
  buildSessionRecoveryRevision,
  clearCctpSession,
  clearPendingEvmTx,
  loadCctpSession,
  patchCctpSessionRecovery,
  purgeCctpSessionIfTerminal,
  saveCctpSession,
  sessionRecoveryMatchesInputs,
  sessionRequiresBindingRecovery,
  setPendingEvmTx,
  type BurnPrepareStep,
  type CctpSessionRecord,
} from '@/lib/cctp/session-vault';
import {
  assessWalletRoleBindings,
  buildWalletRoleBindings,
  signingIntentForBurnStep,
  signingIntentForMintPayload,
  type WalletRoleMismatch,
} from '@/lib/cctp/wallet-role-binding';
import {
  executePreparedPayload,
  reconcileEvmTransactionHash,
} from '@/lib/cctp/wallet-execution';
import { submitBurnWithVerificationRetry } from '@/lib/cctp/stellar-submit';
import { startCctpStatusPoll, type StatusPollHandle } from '@/lib/cctp/status-poll';
import type {
  CctpPrepareBurnResponse,
  CctpQuoteResponse,
  CctpTransferStatus,
  CctpTransferStatusResponse,
} from '@/lib/cctp/types';

export type CctpSagaStage =
  | 'idle'
  | 'quoting'
  | 'quoted'
  | 'sign_approval'
  | 'sign_burn'
  | 'sign_trustline'
  | 'sign_mint'
  | 'awaiting_attestation'
  | 'completed'
  | 'failed'
  | 'unavailable'
  | 'resume_pending'
  | 'pending_reconcile';

export interface CctpWalletRoles {
  sourceStellarAdapterId?: string;
  sourceEvmAdapterId?: string;
  evmDestinationAdapterId?: string;
  mintSubmitterStellarAdapterId?: string;
  sourceAddress?: string;
  recipient: string;
  mintSubmitter?: string;
}

export interface UseCctpSagaInput {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  amount: string;
  recipient: string;
  sender?: string;
  mintSubmitter?: string;
  wallets: CctpWalletRoles;
  bridgeReady: boolean;
  quoteInputsKey: string;
}

const TERMINAL_STAGES = new Set<CctpSagaStage>([
  'idle',
  'completed',
  'failed',
  'unavailable',
]);

function isPreparedPayloadUsable(
  prepared: CctpPrepareBurnResponse | null,
  expectedFingerprint: string | null,
  nowSec = Math.floor(Date.now() / 1000),
): boolean {
  if (!prepared) return false;
  if (prepared.expires_at <= nowSec) return false;
  const fingerprint = fingerprintPreparedPayload(prepared.payload);
  if (expectedFingerprint && fingerprint !== expectedFingerprint) return false;
  return true;
}

function resolveVaultBurnPrepareStep(
  vaultStep: BurnPrepareStep | undefined,
  prepared: CctpPrepareBurnResponse | null,
  fingerprint: string | null,
): BurnPrepareStep {
  if (!vaultStep || vaultStep === 'unknown' || vaultStep === 'reprepare_required') {
    return vaultStep ?? 'unknown';
  }
  if (vaultStep === 'approval_ready' || vaultStep === 'burn_ready') {
    return isPreparedPayloadUsable(prepared, fingerprint) ? vaultStep : 'reprepare_required';
  }
  return vaultStep;
}

function effectiveBurnPrepareStep(
  step: BurnPrepareStep,
  prepared: CctpPrepareBurnResponse | null,
  fingerprint: string | null,
): BurnPrepareStep {
  if (step === 'approval_ready' || step === 'burn_ready') {
    return isPreparedPayloadUsable(prepared, fingerprint) ? step : 'reprepare_required';
  }
  return step;
}

export function useCctpSaga(input: UseCctpSagaInput) {
  const client = useMemo(() => getCctpApiClient(), []);
  const inputRef = useRef(input);
  useEffect(() => {
    inputRef.current = input;
  });
  const [stage, setStage] = useState<CctpSagaStage>('idle');
  const [quote, setQuote] = useState<CctpQuoteResponse | null>(null);
  const [transferStatus, setTransferStatus] =
    useState<CctpTransferStatusResponse | null>(null);
  const [error, setError] = useState<CctpTraderError | null>(null);
  const [session, setSession] = useState<CctpSessionRecord | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [burnPrepareStep, setBurnPrepareStep] = useState<BurnPrepareStep>('unknown');
  const [resumeMismatch, setResumeMismatch] = useState(false);
  const [reattestCooldownUntil, setReattestCooldownUntil] = useState<number | null>(
    null,
  );
  const pollRef = useRef<StatusPollHandle | null>(null);
  const lastInputsKey = useRef<string | null>(null);
  const walletRequestCount = useRef(0);
  const prepareBurnCallCount = useRef(0);
  const [preparedPayload, setPreparedPayload] =
    useState<CctpPrepareBurnResponse | null>(null);
  const [preparedFingerprint, setPreparedFingerprint] = useState<string | null>(
    null,
  );
  const autoReconciledRevisionRef = useRef<string | null>(null);
  const reconcileAbortRef = useRef<AbortController | null>(null);

  const stopPoll = useCallback(() => {
    pollRef.current?.stop();
    pollRef.current = null;
  }, []);

  const abortExpiredQuote = useCallback(() => {
    stopPoll();
    clearCctpSession();
    setSession(null);
    setQuote(null);
    setTransferStatus(null);
    setPreparedPayload(null);
    setPreparedFingerprint(null);
    setBurnPrepareStep('unknown');
    setStage('idle');
  }, [stopPoll]);

  const applyCctpFailure = useCallback(
    (err: unknown, options?: { failedStage?: CctpSagaStage }) => {
      const mapped = mapCctpError(err);
      setError(mapped);
      if (mapped.kind === 'quote_expired') {
        abortExpiredQuote();
        return mapped;
      }
      if (options?.failedStage) {
        setStage(options.failedStage);
      }
      return mapped;
    },
    [abortExpiredQuote],
  );

  useEffect(() => () => {
    stopPoll();
    reconcileAbortRef.current?.abort();
  }, [stopPoll]);

  const syncSession = useCallback((record: CctpSessionRecord | null) => {
    setSession(record);
    if (record?.recovery.pendingEvmTx) {
      setStage('pending_reconcile');
    }
    if (record?.recovery.lastPreparedFingerprint) {
      setPreparedFingerprint(record.recovery.lastPreparedFingerprint);
    }
  }, []);

  const applyResolvedBurnStep = useCallback(
    (record: CctpSessionRecord, prepared: CctpPrepareBurnResponse | null) => {
      const resolved = resolveVaultBurnPrepareStep(
        record.recovery.burnPrepareStep,
        prepared,
        record.recovery.lastPreparedFingerprint ?? preparedFingerprint,
      );
      setBurnPrepareStep(resolved);
      if (
        resolved === 'reprepare_required' &&
        record.recovery.burnPrepareStep &&
        record.recovery.burnPrepareStep !== 'reprepare_required'
      ) {
        const patched = patchCctpSessionRecovery({ burnPrepareStep: 'reprepare_required' });
        if (patched) setSession(patched);
      }
      return resolved;
    },
    [preparedFingerprint],
  );

  const inputsLocked = useMemo(
    () =>
      Boolean(session) &&
      !['idle', 'completed', 'failed', 'unavailable'].includes(stage),
    [session, stage],
  );

  const resolveWalletSigningIntent = useCallback((): Parameters<
    typeof assessWalletRoleBindings
  >[0]['intent'] | null => {
    if (!session) return null;
    if (
      transferStatus?.status === 'attestation_ready' ||
      transferStatus?.status === 'mint_prepared' ||
      transferStatus?.status === 'mint_failed_retryable' ||
      stage === 'sign_mint'
    ) {
      return session.recovery.direction === 'evm_to_stellar'
        ? 'stellar_mint'
        : 'evm_mint';
    }
    if (effectiveBurnPrepareStep(burnPrepareStep, preparedPayload, preparedFingerprint ?? session.recovery.lastPreparedFingerprint ?? null) === 'approval_ready') {
      return 'source_approval';
    }
    if (effectiveBurnPrepareStep(burnPrepareStep, preparedPayload, preparedFingerprint ?? session.recovery.lastPreparedFingerprint ?? null) === 'burn_ready') {
      return 'source_burn';
    }
    if (inputsLocked) return 'resume';
    return null;
  }, [
    burnPrepareStep,
    inputsLocked,
    preparedFingerprint,
    preparedPayload,
    session,
    stage,
    transferStatus?.status,
  ]);

  const requireWalletRoles = useCallback(
    (
      intent: Parameters<typeof assessWalletRoleBindings>[0]['intent'],
      payload?: Parameters<typeof assessWalletRoleBindings>[0]['payload'],
    ): boolean => {
      if (!session) return false;
      const assessment = assessWalletRoleBindings({
        bindings: session.recovery.walletBindings,
        wallets: inputRef.current.wallets,
        intent,
        payload,
      });
      return assessment.ok;
    },
    [session],
  );

  const walletRoleMismatch = useMemo((): WalletRoleMismatch | null => {
    if (!session) return null;
    if (sessionRequiresBindingRecovery(session)) {
      return {
        code: 'bindings_missing',
        role: 'session',
        message:
          'This saved transfer predates wallet verification. Start a new quote to continue.',
      };
    }
    const intent = resolveWalletSigningIntent();
    if (!intent) return null;
    const assessment = assessWalletRoleBindings({
      bindings: session.recovery.walletBindings,
      wallets: input.wallets,
      intent,
    });
    return assessment.ok ? null : assessment.issue;
  }, [input.wallets, resolveWalletSigningIntent, session]);

  useEffect(() => {
    if (
      lastInputsKey.current !== null &&
      lastInputsKey.current !== input.quoteInputsKey &&
      !inputsLocked
    ) {
      setIdempotencyKey(crypto.randomUUID());
      setQuote(null);
      setBurnPrepareStep('unknown');
      if (stage === 'quoted') setStage('idle');
    }
    lastInputsKey.current = input.quoteInputsKey;
  }, [input.quoteInputsKey, inputsLocked, stage]);

  useEffect(() => {
    const loaded = loadCctpSession();
    if (!loaded.ok) return;
    syncSession(loaded.record);
    setIdempotencyKey(loaded.record.idempotencyKey);
    if (!sessionRecoveryMatchesInputs(loaded.record, inputRef.current)) {
      setResumeMismatch(true);
      setStage('resume_pending');
      applyResolvedBurnStep(loaded.record, preparedPayload);
      return;
    }
    applyResolvedBurnStep(loaded.record, null);
    setStage(
      loaded.record.recovery.pendingEvmTx ? 'pending_reconcile' : 'resume_pending',
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps -- mount-only session restore
  }, []);

  const accessOptions = useCallback(() => {
    if (!session) return undefined;
    return { accessToken: session.accessToken };
  }, [session]);

  const applyReattestCooldown = useCallback(
    (status: CctpTransferStatusResponse) => {
      if (status.reattest_cooldown_until) {
        setReattestCooldownUntil(status.reattest_cooldown_until * 1000);
      }
    },
    [],
  );

  const handleStatus = useCallback(
    (status: CctpTransferStatusResponse) => {
      // Successful polls clear transient Iris/network banners so attestation wait
      // does not look frozen after a single 503/blip.
      setError(null);
      setTransferStatus(status);
      applyReattestCooldown(status);
      mapStageFromStatus(status.status, setStage);
      purgeCctpSessionIfTerminal(status.status);
      if (status.status === 'completed') {
        stopPoll();
        clearCctpSession();
        setSession(null);
        setBurnPrepareStep('unknown');
      }
    },
    [applyReattestCooldown, stopPoll],
  );

  const startPoll = useCallback(
    (transferId: string, token: string) => {
      stopPoll();
      pollRef.current = startCctpStatusPoll({
        client,
        transferId,
        accessToken: token,
        callbacks: {
          onUpdate: handleStatus,
          onError: (err) => {
            // Keep polling; only surface soft/transient copy while awaiting Circle.
            setError(mapCctpError(err));
          },
        },
        maxMs: 45 * 60 * 1000,
      });
    },
    [client, handleStatus, stopPoll],
  );

  const persistBurnPrepare = useCallback(
    (prepared: CctpPrepareBurnResponse) => {
      const step: BurnPrepareStep = prepared.approval_required
        ? 'approval_ready'
        : 'burn_ready';
      const fingerprint = fingerprintPreparedPayload(prepared.payload);
      setPreparedFingerprint(fingerprint);
      setPreparedPayload(prepared);
      setBurnPrepareStep(step);
      const patched = patchCctpSessionRecovery({
        burnPrepareStep: step,
        lastPreparedFingerprint: fingerprint,
      });
      if (patched) setSession(patched);
      return step;
    },
    [],
  );

  const requireReprepareAfterSequenceError = useCallback((err: unknown) => {
    if (!isStaleSequenceError(err)) return false;
    setPreparedPayload(null);
    setPreparedFingerprint(null);
    setBurnPrepareStep('reprepare_required');
    patchCctpSessionRecovery({
      burnPrepareStep: 'reprepare_required',
      lastPreparedFingerprint: undefined,
    });
    setError(mapCctpError(err));
    setStage('quoted');
    return true;
  }, []);

  const requestQuote = useCallback(async () => {
    if (!input.bridgeReady) {
      setStage('unavailable');
      setError(
        mapCctpError(
          new StellarRouteApiError(503, 'cctp_not_enabled', 'CCTP not enabled'),
        ),
      );
      return;
    }
    setBusy(true);
    setError(null);
    setStage('quoting');
    setBurnPrepareStep('unknown');
    const key = idempotencyKey ?? crypto.randomUUID();
    setIdempotencyKey(key);

    let body = buildCctpQuoteRequest({
      sourceChainId: input.sourceChainId,
      destChainId: input.destChainId,
      amount: input.amount,
      recipient: input.recipient,
      sender: input.sender,
      mintSubmitter: input.mintSubmitter,
    });
    if (!body) {
      setError({
        kind: 'nonretryable',
        title: 'Unsupported corridor',
        message: 'This chain pair is not a CCTP corridor.',
      });
      setStage('failed');
      setBusy(false);
      return;
    }

    const mintSubmitterForQuote =
      input.mintSubmitter ??
      (body.direction === 'evm_to_stellar' &&
      classifyStellarRecipient(body.recipient) === 'stellar_g'
        ? body.recipient
        : undefined);
    if (body.direction === 'evm_to_stellar' && mintSubmitterForQuote) {
      body = { ...body, mint_submitter: mintSubmitterForQuote };
    }

    const walletBindings = buildWalletRoleBindings({
      direction: body.direction,
      sourceChainId: body.source_chain_id,
      destChainId: body.destination_chain_id,
      sender: body.sender,
      recipient: body.recipient,
      mintSubmitter: body.mint_submitter,
    });
    if (!walletBindings) {
      setError({
        kind: 'nonretryable',
        title: 'Connect required wallets',
        message:
          'Connect the source signer and any required mint submitter wallets before requesting a quote.',
      });
      setStage('failed');
      setBusy(false);
      return;
    }

    try {
      const response = await client.quote(body, { idempotencyKey: key });
      setQuote(response);
      const record = buildCctpSessionRecord({
        transferId: response.transfer_id,
        accessToken: response.access_token,
        idempotencyKey: key,
        quoteExpiresAt: response.expires_at,
        recovery: {
          corridorId: response.corridor_id,
          direction: response.direction,
          sourceChainId: input.sourceChainId,
          destChainId: input.destChainId,
          amount: input.amount,
          recipient: input.recipient,
          quoteExpiresAt: response.expires_at,
          burnPrepareStep: 'unknown',
          walletBindings,
        },
      });
      saveCctpSession(record);
      syncSession(record);
      setResumeMismatch(false);
      setStage('quoted');
    } catch (err) {
      setError(mapCctpError(err));
      setStage('failed');
    } finally {
      setBusy(false);
    }
  }, [client, idempotencyKey, input, syncSession]);

  const prepareSourceBurn = useCallback(async () => {
    if (!session) return;
    if (
      session.recovery.walletBindings &&
      !requireWalletRoles('resume')
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      prepareBurnCallCount.current += 1;
      const prepared = await client.prepareBurn(session.transferId, accessOptions());
      persistBurnPrepare(prepared);
      setStage('quoted');
    } catch (err) {
      applyCctpFailure(err);
    } finally {
      setBusy(false);
    }
  }, [accessOptions, client, persistBurnPrepare, requireWalletRoles, session, applyCctpFailure]);

  const autoReconcileRevision = useMemo(() => {
    if (!session) return null;
    return buildAutoReconcileRevision(session);
  }, [session]);

  const fetchTransferStatus = useCallback(
    async (
      record: CctpSessionRecord,
      options?: { signal?: AbortSignal; force?: boolean },
    ) => {
      const status = await client.getTransfer(record.transferId, {
        accessToken: record.accessToken,
        signal: options?.signal,
      });
      setTransferStatus((prev) =>
        prev?.status === status.status && prev?.transfer_id === status.transfer_id
          ? prev
          : status,
      );
      applyReattestCooldown(status);
      mapStageFromStatus(status.status, setStage);
      purgeCctpSessionIfTerminal(status.status);
      if (status.status === 'completed') {
        stopPoll();
        clearCctpSession();
        setSession(null);
        setBurnPrepareStep('unknown');
        setPreparedPayload(null);
        setPreparedFingerprint(null);
      }
      return status;
    },
    [applyReattestCooldown, client, stopPoll],
  );

  const autoReconcileOnce = useCallback(async () => {
    const loaded = loadCctpSession();
    if (!loaded.ok) {
      if (loaded.reason === 'expired' || loaded.reason === 'invalid') {
        setError({
          kind: 'authorization_lost',
          title: 'Session expired',
          message: 'Start a new quote to continue.',
        });
      }
      return;
    }
    const revision = buildAutoReconcileRevision(loaded.record);
    if (autoReconciledRevisionRef.current === revision) return;

    if (!sessionRecoveryMatchesInputs(loaded.record, inputRef.current)) {
      syncSession(loaded.record);
      setResumeMismatch(true);
      setStage('resume_pending');
      applyResolvedBurnStep(loaded.record, preparedPayload);
      return;
    }

    autoReconciledRevisionRef.current = revision;
    reconcileAbortRef.current?.abort();
    const controller = new AbortController();
    reconcileAbortRef.current = controller;

    try {
      const status = await fetchTransferStatus(loaded.record, {
        signal: controller.signal,
      });
      if (controller.signal.aborted) return;
      const freshLoaded = loadCctpSession();
      const activeRecord = freshLoaded.ok ? freshLoaded.record : loaded.record;
      syncSession(activeRecord);
      applyResolvedBurnStep(activeRecord, preparedPayload);
      setError(null);
      if (activeRecord.recovery.pendingEvmTx) {
        setStage('pending_reconcile');
      } else if (!['completed', 'cancelled', 'provider_killed'].includes(status.status)) {
        setStage((prev) => (prev === 'failed' ? prev : 'quoted'));
      }
      setResumeMismatch(false);
      if (!['completed', 'cancelled', 'provider_killed'].includes(status.status)) {
        if (
          [
            'burn_submitted',
            'awaiting_attestation',
            'attestation_ready',
            'mint_prepared',
            'mint_submitted',
            'attestation_failed',
            'mint_failed_retryable',
          ].includes(status.status)
        ) {
          startPoll(loaded.record.transferId, loaded.record.accessToken);
        }
      }
    } catch (err) {
      if (controller.signal.aborted) return;
      setError({
        kind: 'authorization_lost',
        title: 'Cannot resume transfer',
        message:
          'Start a new quote — the prior access token is no longer valid.',
      });
      clearCctpSession();
      setSession(null);
      setStage('idle');
      autoReconciledRevisionRef.current = null;
    }
  }, [
    applyResolvedBurnStep,
    fetchTransferStatus,
    preparedPayload,
    startPoll,
    syncSession,
  ]);

  const resumeTransfer = useCallback(async () => {
    const loaded = loadCctpSession();
    if (!loaded.ok) {
      if (loaded.reason === 'expired' || loaded.reason === 'invalid') {
        setError({
          kind: 'authorization_lost',
          title: 'Session expired',
          message: 'Start a new quote to continue.',
        });
      }
      return;
    }
    syncSession(loaded.record);
    if (!sessionRecoveryMatchesInputs(loaded.record, inputRef.current)) {
      setResumeMismatch(true);
      setStage('resume_pending');
      applyResolvedBurnStep(loaded.record, preparedPayload);
      return;
    }
    setBusy(true);
    setError(null);
    reconcileAbortRef.current?.abort();
    const controller = new AbortController();
    reconcileAbortRef.current = controller;
    try {
      const status = await fetchTransferStatus(loaded.record, {
        signal: controller.signal,
        force: true,
      });
      if (controller.signal.aborted) return;
      applyResolvedBurnStep(loaded.record, preparedPayload);
      if (loaded.record.recovery.pendingEvmTx) {
        setStage('pending_reconcile');
      } else if (!['completed', 'cancelled', 'provider_killed'].includes(status.status)) {
        setStage('quoted');
      }
      setResumeMismatch(false);
      if (!['completed', 'cancelled', 'provider_killed'].includes(status.status)) {
        if (
          [
            'burn_submitted',
            'awaiting_attestation',
            'attestation_ready',
            'mint_prepared',
            'mint_submitted',
            'attestation_failed',
            'mint_failed_retryable',
          ].includes(status.status)
        ) {
          startPoll(loaded.record.transferId, loaded.record.accessToken);
        }
      }
    } catch {
      if (controller.signal.aborted) return;
      setError({
        kind: 'authorization_lost',
        title: 'Cannot resume transfer',
        message:
          'Start a new quote — the prior access token is no longer valid.',
      });
      clearCctpSession();
      setSession(null);
      setStage('idle');
      autoReconciledRevisionRef.current = null;
    } finally {
      setBusy(false);
    }
  }, [applyResolvedBurnStep, fetchTransferStatus, preparedPayload, startPoll, syncSession]);

  useEffect(() => {
    if (!input.bridgeReady || !autoReconcileRevision) return;
    void autoReconcileOnce();
    // autoReconcileOnce is internally deduped; omit from deps to avoid identity churn.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoReconcileRevision, input.bridgeReady, input.quoteInputsKey]);

  const commitPendingEvmTx = useCallback(
    (input: Parameters<typeof setPendingEvmTx>[0]) => {
      const patched = setPendingEvmTx(input);
      if (patched) {
        syncSession(patched);
        autoReconciledRevisionRef.current = buildAutoReconcileRevision(patched);
      }
      setStage('pending_reconcile');
    },
    [syncSession],
  );

  const resolveEvmAdapterForPayload = useCallback(
    (payloadType: string) => {
      if (payloadType === 'evm_transaction') {
        return (
          input.wallets.sourceEvmAdapterId ??
          input.wallets.evmDestinationAdapterId
        );
      }
      return undefined;
    },
    [input.wallets],
  );

  const signApprovalStep = useCallback(async () => {
    if (!session) {
      setError({
        kind: 'authorization_lost',
        title: 'No active transfer',
        message: 'Request a quote first.',
      });
      return;
    }
    if (!requireWalletRoles(signingIntentForBurnStep('approval_ready'))) {
      return;
    }
    const prepared = preparedPayload;
    const effectiveStep = effectiveBurnPrepareStep(
      burnPrepareStep,
      prepared,
      preparedFingerprint,
    );
    if (
      !prepared?.approval_required ||
      effectiveStep !== 'approval_ready'
    ) {
      setError({
        kind: 'nonretryable',
        title: 'Prepare approval first',
        message: 'Prepare the source transaction before approving USDC spend.',
      });
      return;
    }
    setBusy(true);
    setError(null);
    try {
      setStage('sign_approval');
      walletRequestCount.current += 1;
      const exec = await executePreparedPayload({
        payload: prepared.payload,
        stellarAdapterId: input.wallets.sourceStellarAdapterId,
        evmAdapterId: resolveEvmAdapterForPayload(prepared.payload.type),
        expiresAtSec: prepared.expires_at,
        walletNetwork: 'testnet',
      });
      if (!exec.submissionReady) {
        commitPendingEvmTx({
          txHash: exec.txHash,
          purpose: 'approval',
        });
        return;
      }
      await submitBurnWithVerificationRetry(
        client,
        session.transferId,
        exec.txHash,
        accessOptions(),
        'testnet',
      );
      clearPendingEvmTx();
      const status = await client.getTransfer(session.transferId, accessOptions());
      handleStatus(status);
      setPreparedPayload(null);
      setPreparedFingerprint(null);
      setBurnPrepareStep('unknown');
      patchCctpSessionRecovery({
        burnPrepareStep: 'unknown',
        lastPreparedFingerprint: undefined,
      });
      setError(null);
      const burnPrepared = await client.prepareBurn(
        session.transferId,
        accessOptions(),
      );
      persistBurnPrepare(burnPrepared);
      setStage('quoted');
    } catch (err) {
      if (requireReprepareAfterSequenceError(err)) return;
      applyCctpFailure(err, { failedStage: 'failed' });
    } finally {
      setBusy(false);
    }
  }, [
    accessOptions,
    burnPrepareStep,
    client,
    handleStatus,
    input.wallets,
    preparedFingerprint,
    preparedPayload,
    requireReprepareAfterSequenceError,
    resolveEvmAdapterForPayload,
    session,
    syncSession,
    commitPendingEvmTx,
    requireWalletRoles,
    applyCctpFailure,
    persistBurnPrepare,
  ]);

  const signBurnStep = useCallback(async () => {
    if (!session) {
      setError({
        kind: 'authorization_lost',
        title: 'No active transfer',
        message: 'Request a quote first.',
      });
      return;
    }
    if (!requireWalletRoles(signingIntentForBurnStep('burn_ready'))) {
      return;
    }
    const prepared = preparedPayload;
    const effectiveStep = effectiveBurnPrepareStep(
      burnPrepareStep,
      prepared,
      preparedFingerprint,
    );
    if (!prepared || prepared.approval_required || effectiveStep !== 'burn_ready') {
      setError({
        kind: 'nonretryable',
        title: 'Prepare burn first',
        message: 'Prepare the source transaction before signing the burn.',
      });
      return;
    }
    setBusy(true);
    setError(null);
    try {
      setStage('sign_burn');
      walletRequestCount.current += 1;
      const exec = await executePreparedPayload({
        payload: prepared.payload,
        stellarAdapterId: input.wallets.sourceStellarAdapterId,
        evmAdapterId: resolveEvmAdapterForPayload(prepared.payload.type),
        expiresAtSec: prepared.expires_at,
        walletNetwork: 'testnet',
      });
      if (!exec.submissionReady) {
        commitPendingEvmTx({ txHash: exec.txHash, purpose: 'burn' });
        return;
      }
      await submitBurnWithVerificationRetry(
        client,
        session.transferId,
        exec.txHash,
        accessOptions(),
        'testnet',
      );
      clearPendingEvmTx();
      setPreparedPayload(null);
      setBurnPrepareStep('unknown');
      patchCctpSessionRecovery({ burnPrepareStep: 'unknown' });
      setStage('awaiting_attestation');
      startPoll(session.transferId, session.accessToken);
    } catch (err) {
      if (requireReprepareAfterSequenceError(err)) return;
      applyCctpFailure(err, { failedStage: 'failed' });
    } finally {
      setBusy(false);
    }
  }, [
    accessOptions,
    burnPrepareStep,
    client,
    commitPendingEvmTx,
    input.wallets,
    preparedFingerprint,
    preparedPayload,
    requireReprepareAfterSequenceError,
    resolveEvmAdapterForPayload,
    session,
    startPoll,
    syncSession,
    requireWalletRoles,
    applyCctpFailure,
  ]);

  const signPreparedMintStep = useCallback(async () => {
    if (!session) return;
    setBusy(true);
    setError(null);
    try {
      let prepared = await client.prepareMint(session.transferId, accessOptions());

      // Auto-open USDC trustline when prepare-mint returns trustline_required.
      if (prepared.trustline_required && prepared.payload.type === 'stellar_xdr') {
        if (
          !requireWalletRoles(
            signingIntentForMintPayload(prepared.payload, true),
            prepared.payload,
          )
        ) {
          return;
        }
        setStage('sign_trustline');
        walletRequestCount.current += 1;
        const trustlineAdapterId =
          input.wallets.mintSubmitterStellarAdapterId ??
          input.wallets.sourceStellarAdapterId;
        const trustlineExec = await executePreparedPayload({
          payload: prepared.payload,
          stellarAdapterId: trustlineAdapterId,
          expiresAtSec: prepared.expires_at,
          walletNetwork: 'testnet',
        });
        if (!trustlineExec.submissionReady) {
          setError({
            kind: 'nonretryable',
            title: 'Trustline not confirmed',
            message:
              'USDC trustline submission did not confirm. Retry mint after the trustline is open.',
          });
          setStage('failed');
          return;
        }
        // Re-prepare mint after trustline; do not call submitMint with ChangeTrust hash.
        prepared = await client.prepareMint(session.transferId, accessOptions());
        if (prepared.trustline_required) {
          setError({
            kind: 'nonretryable',
            title: 'Trustline still required',
            message:
              'Horizon still reports no USDC trustline. Confirm Freighter signed for the recipient G-account, then retry.',
          });
          setStage('failed');
          return;
        }
      }

      if (!requireWalletRoles(signingIntentForMintPayload(prepared.payload), prepared.payload)) {
        return;
      }
      setStage('sign_mint');
      walletRequestCount.current += 1;
      const stellarMintId =
        input.wallets.mintSubmitterStellarAdapterId ??
        input.wallets.sourceStellarAdapterId;
      const evmMintId = input.wallets.evmDestinationAdapterId;
      const exec = await executePreparedPayload({
        payload: prepared.payload,
        stellarAdapterId:
          prepared.payload.type === 'stellar_xdr' ? stellarMintId : undefined,
        evmAdapterId:
          prepared.payload.type === 'evm_transaction' ? evmMintId : undefined,
        expiresAtSec: prepared.expires_at,
        walletNetwork: 'testnet',
      });
      if (!exec.submissionReady) {
        commitPendingEvmTx({ txHash: exec.txHash, purpose: 'mint' });
        return;
      }
      await client.submitMint(session.transferId, { tx_hash: exec.txHash }, accessOptions());
      clearPendingEvmTx();
      startPoll(session.transferId, session.accessToken);
    } catch (err) {
      setError(mapCctpError(err));
      setStage('failed');
    } finally {
      setBusy(false);
    }
  }, [accessOptions, client, commitPendingEvmTx, input.wallets, requireWalletRoles, session, startPoll, syncSession]);

  const reconcilePendingEvmTx = useCallback(async () => {
    const loaded = loadCctpSession();
    const pending = loaded.ok ? loaded.record.recovery.pendingEvmTx : session?.recovery.pendingEvmTx;
    if (!session || !pending) return;
    setBusy(true);
    setError(null);
    try {
      const exec = await reconcileEvmTransactionHash({ txHash: pending.txHash });
      if (!exec.submissionReady) {
        setStage('pending_reconcile');
        return;
      }
      if (pending.purpose === 'mint') {
        await client.submitMint(session.transferId, { tx_hash: exec.txHash }, accessOptions());
      } else {
        await client.submitBurn(
          session.transferId,
          { tx_hash: exec.txHash },
          accessOptions(),
        );
      }
      clearPendingEvmTx();
      if (pending.purpose === 'approval') {
        const status = await client.getTransfer(session.transferId, accessOptions());
        handleStatus(status);
        setBurnPrepareStep('unknown');
        patchCctpSessionRecovery({ burnPrepareStep: 'unknown' });
        setStage('quoted');
      } else if (pending.purpose === 'burn') {
        setStage('awaiting_attestation');
        startPoll(session.transferId, session.accessToken);
      } else {
        startPoll(session.transferId, session.accessToken);
      }
    } catch (err) {
      setError(mapCctpError(err));
      setStage('failed');
    } finally {
      setBusy(false);
    }
  }, [
    accessOptions,
    client,
    handleStatus,
    session,
    startPoll,
  ]);

  const reattest = useCallback(async () => {
    if (!session) return;
    if (reattestCooldownUntil && Date.now() < reattestCooldownUntil) return;
    setBusy(true);
    setError(null);
    try {
      const result = await client.reattest(session.transferId, accessOptions());
      handleStatus({
        transfer_id: result.transfer_id,
        corridor_id: quote?.corridor_id ?? session.recovery.corridorId,
        provider: quote?.provider ?? 'circle-cctp',
        direction: session.recovery.direction,
        status: result.status,
        retryable: result.retryable,
      });
      startPoll(session.transferId, session.accessToken);
    } catch (err) {
      if (err instanceof StellarRouteApiError && err.status === 409) {
        try {
          const status = await client.getTransfer(session.transferId, accessOptions());
          handleStatus(status);
        } catch {
          // fall through to mapped error
        }
      }
      setError(mapCctpError(err));
    } finally {
      setBusy(false);
    }
  }, [
    accessOptions,
    client,
    handleStatus,
    quote,
    reattestCooldownUntil,
    session,
    startPoll,
  ]);

  const resetSaga = useCallback(() => {
    stopPoll();
    clearCctpSession();
    setSession(null);
    setQuote(null);
    setTransferStatus(null);
    setError(null);
    setStage('idle');
    setIdempotencyKey(null);
    setBurnPrepareStep('unknown');
    setResumeMismatch(false);
    setReattestCooldownUntil(null);
    walletRequestCount.current = 0;
    prepareBurnCallCount.current = 0;
    setPreparedFingerprint(null);
    setPreparedPayload(null);
    autoReconciledRevisionRef.current = null;
    reconcileAbortRef.current?.abort();
  }, [stopPoll]);

  const effectiveBurnStep = effectiveBurnPrepareStep(
    burnPrepareStep,
    preparedPayload,
    preparedFingerprint ?? session?.recovery.lastPreparedFingerprint ?? null,
  );

  const pendingEvmTx = session?.recovery.pendingEvmTx ?? null;

  const primaryAction = useMemo(() => {
    if (!input.bridgeReady) {
      return { label: 'Bridge unavailable', disabled: true, action: 'none' as const };
    }
    if (stage === 'pending_reconcile' || pendingEvmTx) {
      return {
        label: 'Transaction pending — reconcile',
        disabled: busy,
        action: 'reconcile_pending' as const,
      };
    }
    if (stage === 'resume_pending') {
      return {
        label: 'Resume transfer',
        disabled: busy,
        action: 'resume' as const,
      };
    }
    if (stage === 'completed' || transferStatus?.status === 'completed') {
      return {
        label: 'Transfer complete',
        disabled: true,
        action: 'none' as const,
      };
    }
    // Prefer transfer status over local `failed` so a submit-mint error does not
    // demote a recoverable mint_prepared transfer back to "Get CCTP quote".
    if (
      transferStatus?.status === 'attestation_ready' ||
      transferStatus?.status === 'mint_prepared' ||
      transferStatus?.status === 'mint_failed_retryable'
    ) {
      return {
        label: 'Confirm receive on destination',
        disabled: busy || Boolean(walletRoleMismatch),
        action: 'mint' as const,
      };
    }
    if (stage === 'idle' || stage === 'failed') {
      return { label: 'Get quote', disabled: busy, action: 'quote' as const };
    }
    if (stage === 'quoted') {
      if (effectiveBurnStep === 'reprepare_required') {
        return {
          label: 'Re-prepare transaction',
          disabled: busy || Boolean(walletRoleMismatch),
          action: 'prepare' as const,
        };
      }
      if (effectiveBurnStep === 'unknown') {
        return {
          label: 'Prepare source transaction',
          disabled: busy || Boolean(walletRoleMismatch),
          action: 'prepare' as const,
        };
      }
      if (effectiveBurnStep === 'approval_ready') {
        return {
          label: 'Approve USDC spend',
          disabled: busy || Boolean(walletRoleMismatch),
          action: 'approve' as const,
        };
      }
      if (effectiveBurnStep === 'burn_ready') {
        return {
          label: 'Confirm lock on source chain',
          disabled: busy || Boolean(walletRoleMismatch),
          action: 'burn' as const,
        };
      }
      return {
        label: 'Re-prepare transaction',
        disabled: busy || Boolean(walletRoleMismatch),
        action: 'prepare' as const,
      };
    }
    if (transferStatus?.status === 'attestation_failed') {
      const cooldownActive =
        reattestCooldownUntil !== null && Date.now() < reattestCooldownUntil;
      return {
        label: cooldownActive ? 'Retry confirmation (wait…)' : 'Retry confirmation',
        disabled: busy || cooldownActive,
        action: 'reattest' as const,
      };
    }
    if (
      stage === 'awaiting_attestation' ||
      transferStatus?.status === 'awaiting_attestation' ||
      transferStatus?.status === 'burn_submitted'
    ) {
      const evmSource = session?.recovery.direction === 'evm_to_stellar';
      const fast = quote?.finality === 'fast';
      return {
        label: evmSource
          ? fast
            ? 'Waiting for confirmation…'
            : 'Waiting for confirmation (~15–19 min)…'
          : 'Waiting for confirmation…',
        disabled: true,
        action: 'none' as const,
      };
    }
    if (transferStatus?.status === 'mint_submitted') {
      return {
        label: 'Confirming receive…',
        disabled: true,
        action: 'none' as const,
      };
    }
    return { label: 'Waiting…', disabled: true, action: 'none' as const };
  }, [
    busy,
    effectiveBurnStep,
    input.bridgeReady,
    pendingEvmTx,
    quote?.finality,
    reattestCooldownUntil,
    session?.recovery.direction,
    stage,
    transferStatus?.status,
    walletRoleMismatch,
  ]);

  const runPrimaryAction = useCallback(async () => {
    switch (primaryAction.action) {
      case 'quote':
        await requestQuote();
        break;
      case 'prepare':
        await prepareSourceBurn();
        break;
      case 'approve':
        await signApprovalStep();
        break;
      case 'burn':
        await signBurnStep();
        break;
      case 'mint':
        await signPreparedMintStep();
        break;
      case 'reattest':
        await reattest();
        break;
      case 'reconcile_pending':
        await reconcilePendingEvmTx();
        break;
      case 'resume':
        await resumeTransfer();
        break;
      default:
        break;
    }
  }, [
    primaryAction.action,
    prepareSourceBurn,
    reattest,
    reconcilePendingEvmTx,
    requestQuote,
    resumeTransfer,
    signApprovalStep,
    signBurnStep,
    signPreparedMintStep,
  ]);

  return {
    stage,
    quote,
    transferStatus,
    error,
    busy,
    inputsLocked,
    burnPrepareStep: effectiveBurnStep,
    resumeMismatch,
    walletRoleMismatch,
    pendingEvmTx,
    sessionPublic: session
      ? { transferId: session.transferId, recovery: session.recovery }
      : null,
    primaryAction,
    runPrimaryAction,
    requestQuote,
    prepareSourceBurn,
    autoReconcileOnce,
    resumeTransfer,
    reconcileOnLoad: resumeTransfer,
    reconcilePendingEvmTx,
    resetSaga,
    reattestCooldownUntil,
    getWalletRequestCount: () => walletRequestCount.current,
    getPrepareBurnCallCount: () => prepareBurnCallCount.current,
    getLastPreparedFingerprint: () => preparedFingerprint,
    signApprovalStep,
    signBurnStep,
    signPreparedMintStep,
  };
}

function mapStageFromStatus(
  status: CctpTransferStatus,
  setStage: (s: CctpSagaStage) => void,
) {
  switch (status) {
    case 'completed':
      setStage('completed');
      break;
    case 'awaiting_attestation':
    case 'burn_submitted':
      setStage('awaiting_attestation');
      break;
    case 'attestation_ready':
    case 'mint_prepared':
    case 'mint_failed_retryable':
      setStage('sign_mint');
      break;
    case 'attestation_failed':
      setStage('failed');
      break;
    case 'provider_killed':
      setStage('unavailable');
      break;
    default:
      break;
  }
}

export function isCctpSagaTerminal(stage: CctpSagaStage): boolean {
  return TERMINAL_STAGES.has(stage);
}
