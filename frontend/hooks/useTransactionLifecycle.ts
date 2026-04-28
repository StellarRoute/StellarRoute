"use client";

import { useCallback, useRef, useState } from "react";
import { useTransactionHistory } from "./useTransactionHistory";
import { TransactionStatus } from "@/types/transaction";
import type { PathStep } from "@/types";

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
  errorMessage: string | undefined;
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
}

/** Default stub: simulates a successful wallet signature */
async function defaultSignTransaction(xdr: string): Promise<string> {
  await new Promise((resolve) => setTimeout(resolve, 1500));
  return `signed_${xdr}`;
}

/** Default stub: simulates a successful Horizon submission */
async function defaultSubmitTransaction(
  _signedXdr: string
): Promise<{ hash: string }> {
  await new Promise((resolve) => setTimeout(resolve, 2000));
  return { hash: "mock_tx_" + Math.random().toString(36).substring(7) };
}

function isRejectionError(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes("reject") ||
    lower.includes("denied") ||
    lower.includes("user declined")
  );
}

export function useTransactionLifecycle(
  options: UseTransactionLifecycleOptions = {}
): UseTransactionLifecycleResult {
  const {
    deadlineMs = 60_000,
    signTransaction = defaultSignTransaction,
    submitTransaction = defaultSubmitTransaction,
  } = options;

  const [status, setStatus] = useState<TransactionStatus | "review">("review");
  const [txHash, setTxHash] = useState<string | undefined>(undefined);
  const [errorMessage, setErrorMessage] = useState<string | undefined>(
    undefined
  );
  const [tradeParams, setTradeParams] = useState<TradeParams | undefined>(
    undefined
  );

  // Ref to track the current transaction id for history updates
  const txIdRef = useRef<string | undefined>(undefined);
  // Ref to track the deadline timer
  const deadlineTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined
  );
  // Ref to allow cancel() to abort an in-progress signing
  const cancelledRef = useRef(false);

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
      setTradeParams(params);
      setTxHash(undefined);
      setErrorMessage(undefined);

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

      // Step 1: Sign
      let signedXdr: string;
      try {
        signedXdr = await signTransaction("mock_xdr");
      } catch (err: unknown) {
        if (cancelledRef.current) return;
        const msg =
          err instanceof Error ? err.message : "Signature failed";
        const userFacingMsg = isRejectionError(msg)
          ? "Signature rejected. You can try again or dismiss."
          : msg;
        setErrorMessage(userFacingMsg);
        setStatus("failed");
        updateTransactionStatus(tempId, "failed", {
          errorMessage: userFacingMsg,
        });
        return;
      }

      if (cancelledRef.current) return;

      // Step 2: Submit
      setStatus("submitted");
      updateTransactionStatus(tempId, "submitted");

      // Start deadline timer
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
        const result = await submitTransaction(signedXdr);
        clearDeadlineTimer();

        if (cancelledRef.current) return;

        const hash = result.hash;
        setTxHash(hash);
        setStatus("confirmed");
        updateTransactionStatus(tempId, "confirmed", { hash });
      } catch (err: unknown) {
        clearDeadlineTimer();
        if (cancelledRef.current) return;

        const msg =
          err instanceof Error ? err.message : "Transaction submission failed";
        setErrorMessage(msg);
        setStatus("failed");
        updateTransactionStatus(tempId, "failed", { errorMessage: msg });
      }
    },
    [
      signTransaction,
      submitTransaction,
      deadlineMs,
      addTransaction,
      updateTransactionStatus,
      clearDeadlineTimer,
    ]
  );

  const cancel = useCallback(() => {
    if (status === "pending") {
      cancelledRef.current = true;
      clearDeadlineTimer();
      setStatus("review");
      setErrorMessage(undefined);
    }
  }, [status, clearDeadlineTimer]);

  const resubmit = useCallback(async () => {
    if (status === "dropped" && tradeParams) {
      await initiateSwap(tradeParams);
    }
  }, [status, tradeParams, initiateSwap]);

  const tryAgain = useCallback(() => {
    clearDeadlineTimer();
    setStatus("review");
    setErrorMessage(undefined);
    setTxHash(undefined);
    // tradeParams is preserved so the modal can pre-populate
  }, [clearDeadlineTimer]);

  const dismiss = useCallback(() => {
    clearDeadlineTimer();
    setStatus("review");
    setErrorMessage(undefined);
    setTxHash(undefined);
    setTradeParams(undefined);
  }, [clearDeadlineTimer]);

  return {
    status,
    txHash,
    errorMessage,
    tradeParams,
    initiateSwap,
    cancel,
    resubmit,
    tryAgain,
    dismiss,
  };
}
