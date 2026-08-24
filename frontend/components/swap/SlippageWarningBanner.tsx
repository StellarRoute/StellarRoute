"use client";

import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { SlippageAcknowledgmentState } from "@/hooks/useSlippageAcknowledgment";

interface SlippageWarningBannerProps {
  state: SlippageAcknowledgmentState;
}

export function SlippageWarningBanner({ state }: SlippageWarningBannerProps) {
  const { tier, message, acknowledged, acknowledge } = state;

  if (tier === "safe" || !message) return null;

  const isHigh = tier === "high";

  return (
    <div
      role="alert"
      aria-live="assertive"
      className={`flex flex-col gap-2 rounded-lg border p-3 text-sm
        ${isHigh
          ? "border-red-500/40 bg-red-500/10 text-red-600 dark:text-red-400"
          : "border-yellow-500/40 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400"
        }`}
    >
      <div className="flex items-start gap-2">
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
        <p>{message}</p>
      </div>

      {isHigh && !acknowledged && (        <Button
          variant="outline"
          size="sm"
          className="self-start border-red-500/40 text-red-600 hover:bg-red-500/10 dark:text-red-400"
          onClick={acknowledge}
          aria-label="Acknowledge high slippage risk"
        >
          I understand the risk
        </Button>
      )}

      {isHigh && acknowledged && (
        <p className="text-xs opacity-70">Risk acknowledged.</p>
      )}
    </div>
  );
}
