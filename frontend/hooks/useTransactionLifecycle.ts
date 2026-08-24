"use client";

import { useCallback, useRef, useState } from "react";
import { useTransactionHistory } from "./useTransactionHistory";
import { TransactionStatus } from "@/types/transaction";
import type { PathStep } from "@/types";
import {
  dispatchTransactionNotification,
  type NotificationPreference,
} from "@/lib/notifications";
import { XdrBuildError } from "@/lib/wallet/xdr-builder";
import {
  emitSwapFunnelEvent,
  getPriceImpactTier,
  type SwapFunnelPayload,
} from "@/lib/telemetry";
import {
  toLifecycleError,
  type LifecycleError,
} from "@/lib/swap/lifecycle-error";

function funnelPayloadFromTrade(
  params: TradeParams,
  extra: Partial<SwapFunnelPayload> = {},
): SwapFunnelPayload {
  return {
    fromAssetCode: params.fromAsset,
    toAssetCode: params.toAsset,
    hopCount: params.routePath?.length || 1,
    priceImpactTier: getPriceImpactTier(params.priceImpact),
    ...extra,
  };
}

export interface TradeParams {
  fromAsset: string;
  fromAmount: string;
  toAsset: string;
  toAmount: string;
  exchangeRate: string;
  priceImpact: string;
  minReceived: string;
  networkFee: string;
  routePath: PathStep[];
  walletAddress: string;
}

export interface UseTransactionLifecycleResult {
  status: TransactionStatus | "review";
  txHash: string | undefined;
  /** Backward-compatible string message for non-swap consumers. */
  errorMessage: string | undefined;
  /** Structured error preserving API code + allowlisted status. */
  error: LifecycleError | undefined;
  tradeParams: TradeParams | undefined;
  initiateSwap: (params: TradeParams) => Promise<void>;
  cancel: () => void;
  resubmit: () => Promise<void>;
  tryAgain: () => void;
  dismiss: () => void;
}

interface UseTransactionLifecycleOptions {
  /** Milliseconds to wait for Horizon confirmation before transitioning to `dropped`. Default: 60000 */
  deadlineMs?: number;
  /**
   * Injectable sign function — defaults to a stub that simulates signing.
   * Signature: (xdr: string) => Promise<string>
   * Should throw with a message containing "reject", "denied", or "user declined" on user rejection.
   */
  signTransaction?: (xdr: string) => Promise<string>;
  /**
   * Injectable submit function — defaults to a stub that simulates Horizon submission.
   * Signature: (signedXdr: string) => Promise<{ hash: string }>
   */
  submitTransaction?: (signedXdr: string) => Promise<{ hash: string }>;
  /**
   * Optional XDR builder — when provided, builds a real Stellar path-payment
   * transaction from TradeParams before calling signTransaction.
   * When absent and walletAddress is set, the lifecycle fails fast instead of using mock XDR.
   */
  buildXdr?: (params: TradeParams) => Promise<string>;
  /**
   * Notification preference — injected to keep the hook testable without a real settings store.
   * Defaults to { enabled: false } so notifications are opt-in.
   */
  notificationPreference?: NotificationPreference;
}

/** Default stub: simulates a successful wallet signature */
export async function defaultSignTransaction(xdr: string): Promise<string> {
  await new Promise((resolve) => setTimeout(resolve, 1500));
  return `signed_${xdr}`;
}

/** Default stub: simulates a successful Horizon submission */
export async function defaultSubmitTransaction(
  _signedXdr: string
): Promise<{ hash: string }> {
  await new Promise((resolve) => setTimeout(resolve, 2000));
  return { hash: "mock_tx_" + Math.random().toString(36).substring(7) };
}

export function isDefaultSignTransaction(
  fn: (xdr: string) => Promise<string>
): boolean {
  return fn === defaultSignTransaction;
}

export function isDefaultSubmitTransaction(
  fn: (signedXdr: string) => Promise<{ hash: string }>
): boolean {
  return fn === defaultSubmitTransaction;
}

function isRejectionError(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes("reject") ||
    lower.includes("denied") ||
    lower.includes("user declined") ||
    lower.includes("cancel") ||
    lower.includes("cancelled")
  );
}

export function useTransactionLifecycle(
  options: UseTransactionLifecycleOptions = {}
): UseTransactionLifecycleResult {
  const {
    deadlineMs = 60_000,
    signTransaction = defaultSignTransaction,
    submitTransaction = defaultSubmitTransaction,
    buildXdr,
    notificationPreference = { enabled: false },
  } = options;

  const [status, setStatus] = useState<TransactionStatus | "review">("review");
  const [txHash, setTxHash] = useState<string | undefined>(undefined);
  const [errorMessage, setErrorMessage] = useState<string | undefined>(
    undefined
  );
  const [error, setError] = useState<LifecycleError | undefined>(undefined);
  const [tradeParams, setTradeParams] = useState<TradeParams | undefined>(
    undefined
  );

  const clearError = useCallback(() => {
    setErrorMessage(undefined);
    setError(undefined);
  }, []);

  const failWith = useCallback((err: unknown, messageOverride?: string) => {
    const lifecycleErr = toLifecycleError(err);
    if (messageOverride) {
      lifecycleErr.message = messageOverride;
    }
    setError(lifecycleErr);
    setErrorMessage(lifecycleErr.message);
    return lifecycleErr.message;
  }, []);

  // Ref to track the current transaction id for history updates
  const txIdRef = useRef<string | undefined>(undefined);
  // Ref to track the deadline timer
  const deadlineTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined
  );
  // Ref to allow cancel() to abort an in-progress signing
  const cancelledRef = useRef(false);
  /** Preserved after Freighter signs so pending_reconcile can retry submit only. */
  const lastSignedXdrRef = useRef<string | null>(null);

  const { addTransaction, updateTransactionStatus } = useTransactionHistory(
    tradeParams?.walletAddress ?? null
  );

  const clearDeadlineTimer = useCallback(() => {
    if (deadlineTimerRef.current !== undefined) {
      clearTimeout(deadlineTimerRef.current);
      deadlineTimerRef.current = undefined;
    }
  }, []);

  const initiateSwap = useCallback(
    async (params: TradeParams) => {
      cancelledRef.current = false;
      lastSignedXdrRef.current = null;
      setTradeParams(params);
      setTxHash(undefined);
      clearError();

      if (params.walletAddress) {
        if (!buildXdr) {
          failWith(
            new Error(
              "Transaction could not be built. Please refresh and try again.",
            ),
          );
          setStatus("failed");
          emitSwapFunnelEvent(
            "swap_failed",
            funnelPayloadFromTrade(params, { failureStage: "config" }),
          );
          return;
        }
        if (isDefaultSignTransaction(signTransaction)) {
          failWith(new Error("Wallet not ready for signing."));
          setStatus("failed");
          emitSwapFunnelEvent(
            "swap_failed",
            funnelPayloadFromTrade(params, { failureStage: "config" }),
          );
          return;
        }
        if (isDefaultSubmitTransaction(submitTransaction)) {
          failWith(new Error("Transaction submission is not configured."));
          setStatus("failed");
          emitSwapFunnelEvent(
            "swap_failed",
            funnelPayloadFromTrade(params, { failureStage: "config" }),
          );
          return;
        }
      }

      // Generate a temporary id for the pending record
      const tempId = "pending_" + Date.now();
      txIdRef.current = tempId;

      setStatus("pending");
      addTransaction({
        id: tempId,
        timestamp: Date.now(),
        fromAsset: params.fromAsset,
        fromAmount: params.fromAmount,
        toAsset: params.toAsset,
        toAmount: params.toAmount,
        exchangeRate: params.exchangeRate,
        priceImpact: params.priceImpact,
        minReceived: params.minReceived,
        networkFee: params.networkFee,
        routePath: params.routePath,
        status: "pending",
        walletAddress: params.walletAddress,
      });

      // Step 1: Build XDR (validate quote shape, then construct envelope)
      let xdrToSign: string;
      if (buildXdr) {
        try {
          xdrToSign = await buildXdr(params);
        } catch (err: unknown) {
          if (cancelledRef.current) return;
          const msg =
            err instanceof XdrBuildError
              ? failWith(
                  err,
                  `Transaction build failed (${err.code}): ${err.message}`,
                )
              : failWith(err, undefined);
          setStatus("failed");
          emitSwapFunnelEvent(
            "swap_failed",
            funnelPayloadFromTrade(params, { failureStage: "build" }),
          );
          updateTransactionStatus(tempId, "failed", { errorMessage: msg });
          dispatchTransactionNotification(
            {
              status: "failed",
              fromAsset: params.fromAsset,
              fromAmount: params.fromAmount,
              toAsset: params.toAsset,
              toAmount: params.toAmount,
              txId: tempId,
            },
            notificationPreference,
          );
          return;
        }
      } else {
        xdrToSign = "mock_xdr";
      }

      if (cancelledRef.current) return;

      // Step 2: Sign
      let signedXdr: string;
      try {
        signedXdr = await signTransaction(xdrToSign);
        lastSignedXdrRef.current = signedXdr;
      } catch (err: unknown) {
        if (cancelledRef.current) return;
        lastSignedXdrRef.current = null;
        const rawMsg =
          err instanceof Error ? err.message : "Signature failed";
        const userFacingMsg = isRejectionError(rawMsg)
          ? "Signature rejected. You can try again or dismiss."
          : undefined;
        const msg = failWith(err, userFacingMsg);
        setStatus("failed");
        emitSwapFunnelEvent(
          "swap_failed",
          funnelPayloadFromTrade(params, { failureStage: "sign" }),
        );
        updateTransactionStatus(tempId, "failed", {
          errorMessage: msg,
        });
        dispatchTransactionNotification(
          {
            status: "failed",
            fromAsset: params.fromAsset,
            fromAmount: params.fromAmount,
            toAsset: params.toAsset,
            toAmount: params.toAmount,
            txId: tempId,
          },
          notificationPreference,
        );
        return;
      }

      if (cancelledRef.current) return;

      // Step 3: Submit
      setStatus("submitted");
      emitSwapFunnelEvent("swap_submitted", funnelPayloadFromTrade(params));
      updateTransactionStatus(tempId, "submitted");

      // Start deadline timer
      deadlineTimerRef.current = setTimeout(() => {
        setStatus((current) => {
          if (current === "submitted") {
            updateTransactionStatus(tempId, "dropped");
            dispatchTransactionNotification(
              {
                status: "dropped",
                fromAsset: params.fromAsset,
                fromAmount: params.fromAmount,
                toAsset: params.toAsset,
                toAmount: params.toAmount,
                txId: tempId,
              },
              notificationPreference,
            );
            return "dropped";
          }
          return current;
        });
      }, deadlineMs);

      try {
        const result = await submitTransaction(signedXdr);
        clearDeadlineTimer();

        if (cancelledRef.current) return;

        const hash = result.hash;
        setTxHash(hash);
        setStatus("confirmed");
        emitSwapFunnelEvent("swap_finalized", funnelPayloadFromTrade(params));
        updateTransactionStatus(tempId, "confirmed", { hash });
        dispatchTransactionNotification(
          {
            status: "confirmed",
            txHash: hash,
            fromAsset: params.fromAsset,
            fromAmount: params.fromAmount,
            toAsset: params.toAsset,
            toAmount: params.toAmount,
            txId: tempId,
          },
          notificationPreference,
        );
      } catch (err: unknown) {
        clearDeadlineTimer();
        if (cancelledRef.current) return;

        const msg = failWith(err);
        setStatus("failed");
        emitSwapFunnelEvent(
          "swap_failed",
          funnelPayloadFromTrade(params, { failureStage: "submit" }),
        );
        updateTransactionStatus(tempId, "failed", { errorMessage: msg });
        dispatchTransactionNotification(
          {
            status: "failed",
            fromAsset: params.fromAsset,
            fromAmount: params.fromAmount,
            toAsset: params.toAsset,
            toAmount: params.toAmount,
            txId: tempId,
          },
          notificationPreference,
        );
      }
    },
    [
      signTransaction,
      submitTransaction,
      buildXdr,
      deadlineMs,
      notificationPreference,
      addTransaction,
      updateTransactionStatus,
      clearDeadlineTimer,
      clearError,
      failWith,
    ]
  );

  const cancel = useCallback(() => {
    if (status === "pending") {
      cancelledRef.current = true;
      clearDeadlineTimer();
      setStatus("review");
      clearError();
    }
  }, [status, clearDeadlineTimer, clearError]);

  const resubmit = useCallback(async () => {
    // After Horizon broadcast ambiguity the quote stays `submitting` server-side.
    // Retry the same signed envelope — never prepare/sign again.
    if (
      status === "failed" &&
      error?.status === "pending_reconcile" &&
      tradeParams &&
      lastSignedXdrRef.current
    ) {
      cancelledRef.current = false;
      clearError();
      clearDeadlineTimer();
      const tempId = txIdRef.current ?? `pending_${Date.now()}`;
      txIdRef.current = tempId;
      setStatus("submitted");
      updateTransactionStatus(tempId, "submitted");
      deadlineTimerRef.current = setTimeout(() => {
        setStatus((current) => {
          if (current === "submitted") {
            updateTransactionStatus(tempId, "dropped");
            return "dropped";
          }
          return current;
        });
      }, deadlineMs);
      try {
        const result = await submitTransaction(lastSignedXdrRef.current);
        clearDeadlineTimer();
        if (cancelledRef.current) return;
        setTxHash(result.hash);
        setStatus("confirmed");
        updateTransactionStatus(tempId, "confirmed", { hash: result.hash });
      } catch (err: unknown) {
        clearDeadlineTimer();
        if (cancelledRef.current) return;
        const msg = failWith(err);
        setStatus("failed");
        updateTransactionStatus(tempId, "failed", { errorMessage: msg });
      }
      return;
    }

    if (status === "dropped" && tradeParams) {
      await initiateSwap(tradeParams);
    }
  }, [
    status,
    error?.status,
    tradeParams,
    initiateSwap,
    submitTransaction,
    clearError,
    clearDeadlineTimer,
    updateTransactionStatus,
    deadlineMs,
    failWith,
  ]);

  const tryAgain = useCallback(() => {
    // Prefer reconcile retry when we still hold the signed envelope.
    if (
      status === "failed" &&
      error?.status === "pending_reconcile" &&
      lastSignedXdrRef.current
    ) {
      void resubmit();
      return;
    }
    // Permanent failures must start a fresh prepare/sign — drop the old envelope.
    lastSignedXdrRef.current = null;
    clearDeadlineTimer();
    setStatus("review");
    clearError();
    setTxHash(undefined);
    // tradeParams is preserved so the modal can pre-populate
  }, [status, error?.status, resubmit, clearDeadlineTimer, clearError]);

  const dismiss = useCallback(() => {
    clearDeadlineTimer();
    setStatus("review");
    clearError();
    setTxHash(undefined);
    setTradeParams(undefined);
    lastSignedXdrRef.current = null;
  }, [clearDeadlineTimer, clearError]);

  return {
    status,
    txHash,
    errorMessage,
    error,
    tradeParams,
    initiateSwap,
    cancel,
    resubmit,
    tryAgain,
    dismiss,
  };
}
