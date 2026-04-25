import { useMemo, useState, useEffect, useRef } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { PathStep } from "@/types";
import { RouteVisualization } from "./RouteVisualization";
import { CopyButton } from "./CopyButton";
import { ExplorerLink } from "./ExplorerLink";
import { describeTradeRoute } from "@/lib/route-helpers";
import { TransactionStatus } from "@/types/transaction";
import { useFocusTrap } from "@/hooks/useFocusTrap";
import {
  ArrowDown,
  CheckCircle2,
  XCircle,
  Loader2,
  Wallet,
  ChevronRight,
  TriangleAlert,
  AlertCircle,
  Info,
  Clock,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { getSlippageWarningLevel } from "@/lib/slippage";

export interface BatchSwapItem {
  fromAsset: string;
  fromAmount: string;
  toAsset: string;
  toAmount: string;
  exchangeRate: string;
  priceImpact: string;
  routePath: PathStep[];
}

interface TransactionConfirmationModalProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  // Batch details (if provided, individual trade details below are ignored for the main view)
  swaps?: BatchSwapItem[];
  // Individual trade details (legacy / single mode)
  fromAsset?: string;
  fromAmount?: string;
  toAsset?: string;
  toAmount?: string;
  exchangeRate?: string;
  priceImpact?: string;
  minReceived?: string;
  networkFee: string;
  slippageTolerancePct?: number;
  routePath?: PathStep[];
  // Actions
  onConfirm: () => void;
  onCancel: () => void;
  onTryAgain: () => void;
  onResubmit: () => void;
  onDismiss: () => void;
  onDone: () => void;
  confirmDisabled?: boolean;
  confirmDisabledReason?: string;
  // State
  status: TransactionStatus | "review";
  errorMessage?: string;
  txHash?: string;
}

function parseMaybeNumber(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const n = Number(value);
  if (!Number.isFinite(n)) return undefined;
  return n;
}

const STATE_DESCRIPTIONS: Record<TransactionStatus | "review", string> = {
  review: "Review your transaction details before signing.",
  pending: "Waiting for your wallet signature. Please confirm in your wallet.",
  submitted: "Transaction submitted to the network. Waiting for confirmation.",
  confirmed: "Your transaction has been confirmed on the Stellar network.",
  failed: "Your transaction failed. You can try again or dismiss.",
  dropped:
    "Your transaction was not included in a ledger within the time limit.",
};

export function TransactionConfirmationModal({
  isOpen,
  onOpenChange,
  fromAsset,
  fromAmount,
  toAsset,
  toAmount,
  exchangeRate,
  priceImpact,
  minReceived,
  networkFee,
  slippageTolerancePct,
  routePath,
  onConfirm,
  onCancel,
  onTryAgain,
  onResubmit,
  onDismiss,
  onDone,
  confirmDisabled = false,
  confirmDisabledReason,
  status,
  errorMessage,
  txHash,
  swaps,
}: TransactionConfirmationModalProps) {
  const [countdown, setCountdown] = useState(15);
  const [liveMessage, setLiveMessage] = useState("");

  // Focus refs per state
  const confirmBtnRef = useRef<HTMLButtonElement>(null);
  const cancelBtnRef = useRef<HTMLButtonElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const doneBtnRef = useRef<HTMLButtonElement>(null);
  const tryAgainBtnRef = useRef<HTMLButtonElement>(null);
  const resubmitBtnRef = useRef<HTMLButtonElement>(null);

  // Focus trap while modal is open
  useFocusTrap(containerRef, isOpen);

  const priceImpactValue = useMemo(() => {
    if (swaps && swaps.length > 0) {
      return Math.max(...swaps.map((s) => parseFloat(s.priceImpact) || 0));
    }
    return parseFloat(priceImpact || "0") || 0;
  }, [priceImpact, swaps]);

  const isHighPriceImpact = priceImpactValue >= 2;
  const isSeverePriceImpact = priceImpactValue >= 5;

  const slippageWarningLevel = getSlippageWarningLevel(
    slippageTolerancePct ?? null,
  );
  const isHighSlippage = slippageWarningLevel === "high";
  const isLowSlippage = slippageWarningLevel === "low";

  const computedMinReceived = useMemo(() => {
    const toAmountN = parseMaybeNumber(toAmount);
    if (toAmountN === undefined) return undefined;
    if (slippageTolerancePct === undefined) return undefined;

    const slippageFactor = 1 - slippageTolerancePct / 100;
    if (!(slippageFactor >= 0)) return undefined;

    return String(toAmountN * slippageFactor);
  }, [slippageTolerancePct, toAmount]);

  const minReceivedToDisplay = computedMinReceived ?? minReceived;

  // Auto-refresh mock timer during review state
  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    if (isOpen && status === "review") {
      setCountdown(15);
      timer = setInterval(() => {
        setCountdown((prev: number) => {
          if (prev <= 1) return 15;
          return prev - 1;
        });
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [isOpen, status]);

  // Per-state focus management
  useEffect(() => {
    if (!isOpen) return;
    // Small timeout to ensure the DOM has updated before focusing
    const id = setTimeout(() => {
      switch (status) {
        case "review":
          confirmBtnRef.current?.focus();
          break;
        case "pending":
          cancelBtnRef.current?.focus();
          break;
        case "submitted":
          containerRef.current?.focus();
          break;
        case "confirmed":
          doneBtnRef.current?.focus();
          break;
        case "failed":
          tryAgainBtnRef.current?.focus();
          break;
        case "dropped":
          resubmitBtnRef.current?.focus();
          break;
      }
    }, 50);
    return () => clearTimeout(id);
  }, [status, isOpen]);

  // Escape key handler — suppress close during in-flight states
  useEffect(() => {
    if (!isOpen) return;
    const container = containerRef.current;
    if (!container) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (status === "pending" || status === "submitted") {
        event.preventDefault();
        event.stopPropagation();
        setLiveMessage(
          "Transaction in progress. Use the Cancel button to abort.",
        );
      } else {
        // review, confirmed, failed, dropped — allow close
        onOpenChange(false);
      }
    };

    container.addEventListener("keydown", handleKeyDown, true);
    return () => container.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen, status, onOpenChange]);

  const isBatch = swaps && swaps.length > 0;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => {
      // Only allow Dialog's own close mechanism for non-in-flight states
      if (!open && (status === "pending" || status === "submitted")) {
        setLiveMessage(
          "Transaction in progress. Use the Cancel button to abort.",
        );
        return;
      }
      onOpenChange(open);
    }}>
      <DialogContent
        className="sm:max-w-[425px] w-[90vw] sm:w-auto"
        aria-describedby="modal-state-desc"
      >
        {/* Inner container: receives focus trap, tabIndex, and keydown handler */}
        <div
          ref={containerRef}
          tabIndex={-1}
          className="outline-none"
        >
        {/* Visually hidden state description for screen readers */}
        <p id="modal-state-desc" className="sr-only">
          {STATE_DESCRIPTIONS[status]}
        </p>

        {/* aria-live region for transient announcements */}
        <div
          aria-live="polite"
          aria-atomic="true"
          className="sr-only"
        >
          {liveMessage}
        </div>

        {/* REVIEW STATE */}
        {status === "review" && (
          <>
            <DialogHeader>
              <DialogTitle>Confirm Swap</DialogTitle>
              <DialogDescription>
                Review your transaction details before signing.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4 py-4 max-h-[60vh] overflow-y-auto pr-1">
              {/* Batch or Single Swap Summary */}
              {isBatch ? (
                <div className="space-y-4">
                  <div className="flex items-center justify-between px-1">
                    <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                      Batch Swaps ({swaps!.length})
                    </span>
                    <Badge variant="outline" className="text-[10px]">
                      Atomics enabled
                    </Badge>
                  </div>
                  {swaps!.map((swap, i) => (
                    <div
                      key={i}
                      className="p-3 rounded-lg bg-muted/30 border space-y-2 relative overflow-hidden"
                    >
                      <div className="absolute top-0 right-0 p-1 opacity-10">
                        <span className="text-4xl font-black italic">
                          #{i + 1}
                        </span>
                      </div>
                      <div className="flex justify-between items-end relative z-10">
                        <div>
                          <p className="text-[10px] text-muted-foreground uppercase font-bold">
                            Pay
                          </p>
                          <p className="font-bold">
                            {swap.fromAmount} {swap.fromAsset}
                          </p>
                        </div>
                        <div className="text-center pb-1">
                          <ChevronRight className="w-4 h-4 text-muted-foreground/50" />
                        </div>
                        <div className="text-right">
                          <p className="text-[10px] text-muted-foreground uppercase font-bold">
                            Receive
                          </p>
                          <p className="font-bold text-success">
                            {swap.toAmount} {swap.toAsset}
                          </p>
                        </div>
                      </div>
                      <div className="pt-2 border-t border-border/40 flex justify-between items-center text-[10px] text-muted-foreground">
                        <span>Rate: {swap.exchangeRate}</span>
                        <span
                          className={cn(
                            parseFloat(swap.priceImpact) > 1
                              ? "text-destructive"
                              : "text-success"
                          )}
                        >
                          Impact: {swap.priceImpact}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                /* Single Swap Summary */
                <div className="p-4 rounded-lg bg-muted/30 border space-y-3">
                  <div className="flex justify-between items-center">
                    <span className="text-sm font-medium text-muted-foreground">
                      You Pay
                    </span>
                    <div className="text-right">
                      <p className="text-lg font-bold">
                        {fromAmount} {fromAsset}
                      </p>
                    </div>
                  </div>

                  <div className="flex justify-center -my-2 relative z-10">
                    <div className="bg-background border rounded-full p-1">
                      <ArrowDown className="w-4 h-4 text-muted-foreground" />
                    </div>
                  </div>

                  <div className="flex justify-between items-center">
                    <span className="text-sm font-medium text-muted-foreground">
                      You Receive
                    </span>
                    <div className="text-right">
                      <p className="text-lg font-bold text-success">
                        ~{toAmount} {toAsset}
                      </p>
                      <p className="text-[10px] text-muted-foreground uppercase tracking-wider">
                        Estimated Minimum: {minReceivedToDisplay ?? "—"}{" "}
                        {toAsset}
                      </p>
                    </div>
                  </div>
                </div>
              )}

              {/* Warnings Section */}
              {(isHighPriceImpact || isHighSlippage || isLowSlippage) && (
                <div className="space-y-2">
                  {isSeverePriceImpact ? (
                    <div className="flex gap-2 p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-destructive text-xs">
                      <TriangleAlert className="w-4 h-4 shrink-0" />
                      <div>
                        <p className="font-bold">
                          Very High Price Impact ({priceImpact})
                        </p>
                        <p>
                          This trade will significantly move the market price.
                          You may receive much less than expected.
                        </p>
                      </div>
                    </div>
                  ) : isHighPriceImpact ? (
                    <div className="flex gap-2 p-3 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 text-xs">
                      <AlertCircle className="w-4 h-4 shrink-0" />
                      <div>
                        <p className="font-bold">
                          High Price Impact ({priceImpact})
                        </p>
                        <p>
                          The price for this trade is significantly different
                          from the current market rate.
                        </p>
                      </div>
                    </div>
                  ) : null}

                  {isHighSlippage && (
                    <div className="flex gap-2 p-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 text-xs">
                      <Info className="w-4 h-4 shrink-0" />
                      <div>
                        <p className="font-medium text-amber-700 dark:text-amber-300">
                          High Slippage Tolerance ({slippageTolerancePct}%)
                        </p>
                        <p className="opacity-80">
                          Your transaction might be frontrun or you may receive
                          a much worse price.
                        </p>
                      </div>
                    </div>
                  )}

                  {isLowSlippage && (
                    <div className="flex gap-2 p-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 text-xs">
                      <Info className="w-4 h-4 shrink-0" />
                      <div>
                        <p className="font-medium">Very Low Slippage</p>
                        <p className="opacity-80">
                          Transaction might fail if the price moves even
                          slightly before confirmation.
                        </p>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Trade Details */}
              <div className="space-y-2 text-sm">
                {!isBatch && (
                  <>
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">Rate</span>
                      <span>
                        1 {fromAsset} = {exchangeRate} {toAsset}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">
                        Price Impact
                      </span>
                      <span
                        className={
                          parseFloat(priceImpact || "0") > 1
                            ? "text-destructive font-medium"
                            : "text-success font-medium"
                        }
                      >
                        {priceImpact}
                      </span>
                    </div>
                  </>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Slippage</span>
                  <span>
                    {slippageTolerancePct === undefined
                      ? "—"
                      : `${slippageTolerancePct}%`}
                  </span>
                </div>
                {!isBatch && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">
                      Minimum Received
                    </span>
                    <span>
                      {minReceivedToDisplay ?? "—"} {toAsset}
                    </span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Network Fee</span>
                  <span>{networkFee} XLM</span>
                </div>
              <div className="flex flex-col gap-1 pt-2">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Route</span>
                  <CopyButton
                    value={describeTradeRoute(routePath || [])}
                    label="Copy route summary"
                  />
                </div>
                <RouteVisualization
                  path={routePath || []}
                  className="border-none shadow-none bg-transparent p-0"
                />
              </div>
              </div>
            </div>

            <DialogFooter className="flex-col sm:flex-col gap-2">
              <Button
                ref={confirmBtnRef}
                onClick={onConfirm}
                disabled={confirmDisabled}
                className="w-full min-h-[48px]"
                size="lg"
              >
                {isBatch ? "Confirm Batch Swaps" : "Confirm Swap"}
              </Button>
              {confirmDisabledReason && (
                <p className="w-full text-center text-xs text-destructive">
                  {confirmDisabledReason}
                </p>
              )}
              <Button
                type="button"
                variant="outline"
                className="w-full min-h-[48px]"
                onClick={onCancel}
              >
                Cancel
              </Button>
              <div className="text-center text-xs text-muted-foreground">
                Quote refreshes in {countdown}s
              </div>
            </DialogFooter>
          </>
        )}

        {/* AWAITING SIGNATURE STATE */}
        {status === "pending" && (
          <div className="py-12 flex flex-col items-center justify-center space-y-4 text-center">
            <div className="relative">
              <div className="absolute inset-0 bg-primary/20 rounded-full animate-ping" />
              <div className="bg-primary/10 p-4 rounded-full relative">
                <Wallet className="w-12 h-12 text-primary" />
              </div>
            </div>
            <div>
              <DialogTitle className="text-xl mb-2">
                Awaiting Signature
              </DialogTitle>
              <DialogDescription>
                Please confirm the transaction in your wallet to continue.
              </DialogDescription>
            </div>
            <Button
              ref={cancelBtnRef}
              type="button"
              variant="outline"
              className="w-full min-h-[48px]"
              onClick={onCancel}
            >
              Cancel
            </Button>
          </div>
        )}

        {/* SUBMITTED STATE */}
        {status === "submitted" && (
          <div className="py-12 flex flex-col items-center justify-center space-y-4 text-center">
            <Loader2 className="w-16 h-16 text-primary animate-spin" />
            <div>
              <DialogTitle className="text-xl mb-2">
                Submitting to network…
              </DialogTitle>
              <DialogDescription>
                Waiting for network confirmation. This should only take a few
                seconds.
              </DialogDescription>
            </div>
            {txHash && (
              <ExplorerLink
                hash={txHash}
                className="flex items-center gap-1 text-sm text-primary hover:underline"
              />
            )}
          </div>
        )}

        {/* CONFIRMED STATE */}
        {status === "confirmed" && (
          <div className="py-8 flex flex-col items-center justify-center space-y-6 text-center">
            <div className="bg-success/10 p-4 rounded-full">
              <CheckCircle2 className="w-16 h-16 text-success" />
            </div>
            <div>
              <DialogTitle className="text-2xl mb-2">
                {isBatch ? "Batch Confirmed!" : "Swap Confirmed!"}
              </DialogTitle>
              <DialogDescription>
                {swaps && swaps.length > 1 ? (
                  <span>
                    Processed {swaps.length} transactions in one atomic batch.
                  </span>
                ) : (
                  <>
                    You received{" "}
                    <span className="font-bold text-foreground">
                      {toAmount} {toAsset}
                    </span>
                  </>
                )}
              </DialogDescription>
            </div>

            {txHash && (
              <div className="min-h-[44px] flex flex-col items-center gap-2">
                <div className="flex items-center gap-1">
                  <span className="font-mono text-xs text-muted-foreground truncate max-w-[240px]">
                    {txHash}
                  </span>
                  <CopyButton value={txHash} label="Copy transaction hash" />
                </div>
                <ExplorerLink
                  hash={txHash}
                  className="flex items-center gap-1 text-sm text-primary hover:underline"
                />
              </div>
            )}

            <Button
              ref={doneBtnRef}
              onClick={onDone}
              className="w-full mt-4"
            >
              Done
            </Button>
          </div>
        )}

        {/* FAILED STATE */}
        {status === "failed" && (
          <div className="py-8 flex flex-col items-center justify-center space-y-6 text-center">
            <div className="bg-destructive/10 p-4 rounded-full">
              <XCircle className="w-16 h-16 text-destructive" />
            </div>
            <div>
              <DialogTitle className="text-xl mb-2">
                Transaction Failed
              </DialogTitle>
              <DialogDescription className="text-destructive max-w-[280px] mx-auto">
                {errorMessage ||
                  "An unknown error occurred while processing your transaction."}
              </DialogDescription>
            </div>

            <div className="w-full space-y-2 mt-4">
              <Button
                ref={tryAgainBtnRef}
                onClick={onTryAgain}
                className="w-full"
              >
                Try Again
              </Button>
              <Button
                onClick={onDismiss}
                className="w-full"
                variant="outline"
              >
                Dismiss
              </Button>
            </div>
          </div>
        )}

        {/* DROPPED STATE */}
        {status === "dropped" && (
          <div className="py-8 flex flex-col items-center justify-center space-y-6 text-center">
            <div className="bg-muted p-4 rounded-full">
              <Clock className="w-16 h-16 text-muted-foreground" />
            </div>
            <div>
              <DialogTitle className="text-xl mb-2">
                Transaction Dropped
              </DialogTitle>
              <DialogDescription className="max-w-[280px] mx-auto">
                Your transaction was not included in a ledger within the time
                limit.
              </DialogDescription>
            </div>

            <div className="w-full space-y-2 mt-4">
              <Button
                ref={resubmitBtnRef}
                onClick={onResubmit}
                className="w-full"
              >
                Resubmit
              </Button>
              <Button
                onClick={onDismiss}
                className="w-full"
                variant="outline"
              >
                Dismiss
              </Button>
            </div>
          </div>
        )}
        </div>{/* end inner container */}
      </DialogContent>
    </Dialog>
  );
}
