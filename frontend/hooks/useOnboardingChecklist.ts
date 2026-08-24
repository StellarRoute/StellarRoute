"use client";

import { useCallback, useEffect, useState } from "react";

export type OnboardingStepId =
  | "connect_wallet"
  | "pick_pair"
  | "review_impact"
  | "confirm_swap";

export interface OnboardingStep {
  id: OnboardingStepId;
  label: string;
  description: string;
  /** href or element id to scroll/focus to */
  anchor?: string;
}

export const ONBOARDING_STEPS: OnboardingStep[] = [
  {
    id: "connect_wallet",
    label: "Connect your wallet",
    description: "Link Freighter or XBull to start trading.",
    anchor: "#wallet-button",
  },
  {
    id: "pick_pair",
    label: "Pick a trading pair",
    description: "Choose the assets you want to swap.",
    anchor: "#pair-selector",
  },
  {
    id: "review_impact",
    label: "Review price impact",
    description: "Check slippage and route before you proceed.",
    anchor: "#price-impact",
  },
  {
    id: "confirm_swap",
    label: "Confirm your swap",
    description: "Submit the transaction from your wallet.",
    anchor: "#swap-button",
  },
];

const STORAGE_KEY = "stellarroute:onboarding:dismissed";

export function useOnboardingChecklist() {
  const [dismissed, setDismissed] = useState(false);
  const [completed, setCompleted] = useState<Set<OnboardingStepId>>(new Set());

  // Hydrate from localStorage
  useEffect(() => {
    try {
      if (localStorage.getItem(STORAGE_KEY) === "true") {
        setDismissed(true);
      }
    } catch {
      // SSR / private browsing — ignore
    }
  }, []);

  const dismiss = useCallback(() => {
    setDismissed(true);
    try {
      localStorage.setItem(STORAGE_KEY, "true");
    } catch {
      // ignore
    }
  }, []);

  const markComplete = useCallback((step: OnboardingStepId) => {
    setCompleted((prev) => new Set([...prev, step]));
  }, []);

  const reset = useCallback(() => {
    setDismissed(false);
    setCompleted(new Set());
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // ignore
    }
  }, []);

  return { dismissed, completed, dismiss, markComplete, reset };
}
