"use client";

import { useFeatureFlag } from "@/hooks/useFeatureFlag";
import { useReducedMotion } from "@/hooks/useReducedMotion";
import { cn } from "@/lib/utils";
import { ViewState } from "@/components/shared/ViewState";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { ArrowRight, Lock, RefreshCw, AlertTriangle, TrendingDown } from "lucide-react";

export interface BatchSwapLeg {
  id: string;
  fromAsset: string;
  toAsset: string;
  fromAmount: string;
  toAmount: string;
  price?: string;
  priceImpact?: string;
}

export interface BatchSwapPreviewProps {
  legs: BatchSwapLeg[];
  isLoading?: boolean;
  error?: string;
  onConfirm?: () => void;
  onCancel?: () => void;
  onRetry?: () => void;
}

export function BatchSwapPreview({
  legs,
  isLoading = false,
  error,
  onConfirm,
  onCancel,
  onRetry,
}: BatchSwapPreviewProps) {
  const isEnabled = useFeatureFlag("batchSwaps");
  const prefersReducedMotion = useReducedMotion();

  // Gated behind feature flag
  if (!isEnabled) {
    return (
      <Card className={cn(
        'p-6 border border-warning/20 bg-warning/5 backdrop-blur-md rounded-2xl flex flex-col items-center text-center gap-4',
        !prefersReducedMotion && 'animate-in fade-in duration-300'
      )}>
        <div className="h-12 w-12 rounded-full bg-warning/10 flex items-center justify-center text-warning shadow-[0_0_15px_rgba(245,158,11,0.15)]">
          <Lock className="h-5 w-5" />
        </div>
        <div className="space-y-1.5">
          <h3 className="text-base font-semibold tracking-tight text-amber-900 dark:text-amber-100">Batch Swap Beta</h3>
          <p className="text-sm text-amber-800/80 dark:text-amber-200/80 max-w-sm">
            Batch swapping is currently in developer preview. Please enable the <code className="font-mono bg-warning/15 px-1.5 py-0.5 rounded text-xs">batchSwaps</code> feature flag to interact with batch swaps.
          </p>
        </div>
      </Card>
    );
  }

  // Loading state
  if (isLoading) {
    return (
      <div className="space-y-4" data-testid="batch-swap-loading">
        <div className="space-y-2">
          <Skeleton className="h-5 w-32 rounded" />
          <Skeleton className="h-3 w-48 rounded" />
        </div>
        <div className="space-y-3">
          {[1, 2].map((i) => (
            <Card key={i} className="p-4 border border-border/50 space-y-3">
              <div className="flex justify-between items-center">
                <Skeleton className="h-4 w-16 rounded" />
                <Skeleton className="h-4 w-24 rounded" />
              </div>
              <div className="flex items-center gap-3 justify-center py-2">
                <Skeleton className="h-5 w-20 rounded" />
                <Skeleton className="h-4 w-4 rounded-full" />
                <Skeleton className="h-5 w-20 rounded" />
              </div>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <ViewState
        variant="error"
        title="Simulation Failed"
        description={error}
        action={
          onRetry ? (
            <Button
              variant="outline"
              size="sm"
              onClick={onRetry}
              className="gap-2 min-h-[44px] min-w-[100px] rounded-xl hover:bg-destructive/5 transition-colors border-destructive/20 text-destructive hover:text-destructive"
            >
              <RefreshCw className="h-4 w-4" />
              Retry Simulation
            </Button>
          ) : undefined
        }
      />
    );
  }

  // Empty state
  if (legs.length === 0) {
    return (
      <ViewState
        variant="empty"
        title="No Swap Legs Found"
        description="Your batch swap is currently empty. Add operations to start previewing your multi-leg transaction."
      />
    );
  }

  // Calculate subtotals grouped by unique assets
  const inputTotals: Record<string, number> = {};
  const outputTotals: Record<string, number> = {};

  legs.forEach((leg) => {
    const fromVal = parseFloat(leg.fromAmount) || 0;
    const toVal = parseFloat(leg.toAmount) || 0;

    inputTotals[leg.fromAsset] = (inputTotals[leg.fromAsset] || 0) + fromVal;
    outputTotals[leg.toAsset] = (outputTotals[leg.toAsset] || 0) + toVal;
  });

  const uniqueInputs = Object.entries(inputTotals).map(([asset, amount]) => ({
    asset,
    amount: amount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 }),
  }));

  const uniqueOutputs = Object.entries(outputTotals).map(([asset, amount]) => ({
    asset,
    amount: amount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 }),
  }));

  return (
    <div className="space-y-5" data-testid="batch-swap-preview">
      {/* Component Title */}
      <div className="flex justify-between items-baseline border-b border-border/20 pb-3">
        <div>
          <h3 className="text-base font-bold tracking-tight">Batch Swap Preview</h3>
          <p className="text-xs text-muted-foreground">Review your multi-leg batch transaction below</p>
        </div>
        <span className="text-xs font-semibold px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20">
          {legs.length} {legs.length === 1 ? "Leg" : "Legs"}
        </span>
      </div>

      {/* Leg list */}
      <div className="space-y-3 max-h-[320px] overflow-y-auto pr-1">
        {legs.map((leg, index) => {
          const priceImpactVal = leg.priceImpact ? parseFloat(leg.priceImpact) : 0;
          const isHighImpact = priceImpactVal > 2.0;

          return (
            <Card
              key={leg.id}
              className="p-4 border border-border/40 bg-background/50 hover:bg-muted/10 transition-all rounded-xl relative overflow-hidden"
            >
              {/* Leg Counter Badge */}
              <div className="flex justify-between items-center mb-2">
                <span className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                  Leg #{index + 1}
                </span>
                {leg.priceImpact && (
                  <span
                    className={`text-[10px] font-bold px-2 py-0.5 rounded-full flex items-center gap-1 ${
                      isHighImpact
                        ? "bg-destructive/10 text-destructive border border-destructive/20"
                        : "bg-success/10 text-success border border-success/20"
                    }`}
                  >
                    {isHighImpact && <AlertTriangle className="h-2.5 w-2.5" />}
                    Impact: {leg.priceImpact}%
                  </span>
                )}
              </div>

              {/* Legs flow visual */}
              <div className="flex items-center justify-between py-1.5 gap-4">
                <div className="flex flex-col">
                  <span className="text-xs text-muted-foreground">Pay</span>
                  <span className="text-sm font-bold font-mono text-foreground truncate max-w-[100px]">
                    {leg.fromAmount}
                  </span>
                  <span className="text-xs font-semibold text-muted-foreground">{leg.fromAsset}</span>
                </div>

                <div className="flex flex-col items-center justify-center flex-1">
                  <div className={cn(
                  'h-8 w-8 rounded-full bg-muted/40 flex items-center justify-center border border-border/20',
                  !prefersReducedMotion && 'hover:scale-105 transition-transform duration-200'
                )}>
                    <ArrowRight className="h-4 w-4 text-muted-foreground" />
                  </div>
                </div>

                <div className="flex flex-col items-end">
                  <span className="text-xs text-muted-foreground">Receive</span>
                  <span className="text-sm font-bold font-mono text-primary truncate max-w-[100px]">
                    {leg.toAmount}
                  </span>
                  <span className="text-xs font-semibold text-muted-foreground">{leg.toAsset}</span>
                </div>
              </div>

              {/* Leg conversion rate */}
              {leg.price && (
                <div className="mt-3 pt-2 border-t border-border/10 flex justify-between items-center text-[11px] text-muted-foreground font-mono">
                  <span>Exchange Rate</span>
                  <span>
                    1 {leg.fromAsset} ≈ {leg.price} {leg.toAsset}
                  </span>
                </div>
              )}
            </Card>
          );
        })}
      </div>

      {/* Subtotals card */}
      <Card data-testid="batch-subtotals" className="p-4 border border-primary/20 bg-primary/[0.02] backdrop-blur-md rounded-xl space-y-3.5">
        <h4 className="text-xs font-bold uppercase tracking-wider text-muted-foreground border-b border-border/10 pb-1.5">
          Consolidated Subtotals
        </h4>
        
        <div className="grid grid-cols-2 gap-4">
          {/* Total Sold */}
          <div className="space-y-1.5">
            <span className="text-[11px] font-bold text-muted-foreground block">Estimated Total Sent</span>
            <div data-testid="subtotals-sent" className="space-y-1 max-h-[80px] overflow-y-auto pr-1">
              {uniqueInputs.map(({ asset, amount }) => (
                <div key={asset} className="flex items-baseline justify-between font-mono text-xs text-foreground">
                  <span className="font-semibold text-muted-foreground">{asset}</span>
                  <span className="font-bold">{amount}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Total Received */}
          <div className="space-y-1.5 border-l border-border/20 pl-4">
            <span className="text-[11px] font-bold text-muted-foreground block">Estimated Total Received</span>
            <div data-testid="subtotals-received" className="space-y-1 max-h-[80px] overflow-y-auto pr-1">
              {uniqueOutputs.map(({ asset, amount }) => (
                <div key={asset} className="flex items-baseline justify-between font-mono text-xs text-primary">
                  <span className="font-semibold text-muted-foreground">{asset}</span>
                  <span className="font-bold">{amount}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </Card>

      {/* Interactive controls */}
      <div className="flex flex-col sm:flex-row gap-3 pt-2">
        {onCancel && (
          <Button
            variant="outline"
            onClick={onCancel}
            className="flex-1 min-h-[44px] rounded-xl hover:bg-muted/80 font-semibold"
          >
            Clear Batch
          </Button>
        )}
        {onConfirm && (
          <Button
            onClick={onConfirm}
            className="flex-1 bg-primary text-primary-foreground hover:bg-primary/95 min-h-[44px] rounded-xl font-bold gap-2 shadow-lg shadow-primary/10 hover:shadow-primary/20 transition-all duration-300"
          >
            Submit Batch Swap
          </Button>
        )}
      </div>
    </div>
  );
}
