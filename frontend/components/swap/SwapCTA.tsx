"use client";

import { Button } from "@/components/ui/button";
import { Loader2 } from "lucide-react";
import type { SwapValidationResult } from "@/lib/swap-validation";
import { useSwapI18n } from "@/lib/swap-i18n";

interface SwapCTAProps {
  validation: SwapValidationResult;
  isLoading: boolean;
  isOnline?: boolean;
  onSwap: () => void;
}

export function SwapCTA({
  validation,
  isLoading,
  isOnline = true,
  onSwap,
}: SwapCTAProps) {
  const { t } = useSwapI18n();
  let label = t("swap.cta.reviewSwap");
  let disabled = false;

  const hasPairIssue = validation.issues.some((issue) => issue.field === "pair");
  const hasAmountIssue = validation.issues.some(
    (issue) => issue.field === "amount",
  );
  const hasSlippageIssue = validation.issues.some(
    (issue) => issue.field === "slippage",
  );

  if (!isOnline) {
    label = t("swap.cta.offline");
    disabled = true;
  } else if (hasPairIssue) {
    label = t("swap.cta.selectTokens");
    disabled = true;
  } else if (hasAmountIssue) {
    label = t("swap.cta.enterAmount");
    disabled = true;
  } else if (hasSlippageIssue) {
    label = t("swap.cta.invalidSlippage");
    disabled = true;
  } else if (isLoading) {
    label = t("swap.cta.loadingQuote");
    disabled = true;
  }

  return (
    <Button
      className="mt-2 h-14 w-full text-lg font-medium shadow-md transition-all active:scale-[0.98]"
      size="lg"
      disabled={disabled}
      onClick={onSwap}
    >
      {isLoading && <Loader2 className="mr-2 h-5 w-5 animate-spin" />}
      {label}
    </Button>
  );
}
