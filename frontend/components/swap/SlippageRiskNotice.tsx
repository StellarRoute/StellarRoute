'use client';

import { AlertCircle, AlertTriangle } from "lucide-react";

import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import {
  getSlippageWarningTier,
  type SlippageWarningTier,
} from "@/lib/slippage";

const copyByTier: Record<
  SlippageWarningTier,
  { title: string; body: string; tone: string }
> = {
  low: {
    title: "Low slippage tolerance",
    body: "This swap may fail if the price moves before execution.",
    tone: "border-yellow-500/20 bg-yellow-500/10 text-yellow-700 dark:text-yellow-300",
  },
  elevated: {
    title: "Elevated slippage tolerance",
    body: "Review the minimum received amount before continuing.",
    tone: "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  },
  high: {
    title: "High slippage tolerance",
    body: "High slippage can execute at a significantly worse price. Acknowledge this risk to continue.",
    tone: "border-destructive/25 bg-destructive/10 text-destructive",
  },
};

export function SlippageRiskNotice({
  slippage,
  acknowledged,
  onAcknowledgedChange,
}: {
  slippage: number;
  acknowledged: boolean;
  onAcknowledgedChange: (acknowledged: boolean) => void;
}) {
  const tier = getSlippageWarningTier(slippage);

  if (!tier) return null;

  const copy = copyByTier[tier];
  const isHigh = tier === "high";

  return (
    <div
      data-testid="slippage-risk-notice"
      className={cn(
        "rounded-xl border p-3 text-xs",
        copy.tone,
      )}
    >
      <div className="flex items-start gap-2">
        {isHigh ? (
          <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" aria-hidden="true" />
        ) : (
          <AlertTriangle className="mt-0.5 h-4 w-4 flex-shrink-0" aria-hidden="true" />
        )}
        <div className="min-w-0 space-y-1">
          <p className="font-semibold">
            {copy.title} ({slippage}%)
          </p>
          <p>{copy.body}</p>
        </div>
      </div>

      {isHigh ? (
        <label className="mt-3 flex min-h-11 cursor-pointer items-center gap-2 rounded-lg border border-destructive/20 bg-background/70 px-3 py-2 text-foreground">
          <Checkbox
            checked={acknowledged}
            onCheckedChange={(value) => onAcknowledgedChange(value === true)}
            aria-label="Acknowledge high slippage risk"
          />
          <span className="text-xs font-medium">
            I understand this high slippage risk.
          </span>
        </label>
      ) : null}
    </div>
  );
}
