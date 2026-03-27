"use client";

import { Button } from "@/components/ui/button";
import { Loader2 } from "lucide-react";
import type { SwapValidationResult } from "@/lib/swap-validation";

interface SwapCTAProps {
  validation: SwapValidationResult;
  isLoading: boolean;
  onSwap: () => void;
}

export function SwapCTA({ validation, isLoading, onSwap }: SwapCTAProps) {
  let label = "Review Swap";
  let disabled = false;

  const hasPairIssue = validation.issues.some((issue) => issue.field === "pair");
  const hasAmountIssue = validation.issues.some(
    (issue) => issue.field === "amount",
  );
  const hasSlippageIssue = validation.issues.some(
    (issue) => issue.field === "slippage",
  );

  if (hasPairIssue) {
    label = "Select tokens";
    disabled = true;
  } else if (hasAmountIssue) {
    label = "Enter amount";
    disabled = true;
  } else if (hasSlippageIssue) {
    label = "Invalid slippage";
    disabled = true;
  } else if (isLoading) {
    label = "Loading quote...";
    disabled = true;
  }

  return (
    <Button 
      className="w-full h-14 text-lg font-medium shadow-md transition-all active:scale-[0.98] mt-2" 
      size="lg"
      disabled={disabled}
      onClick={onSwap}
    >
      {isLoading && <Loader2 className="mr-2 h-5 w-5 animate-spin" />}
      {label}
    </Button>
  );
}
