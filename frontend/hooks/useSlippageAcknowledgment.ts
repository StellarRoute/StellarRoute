"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { getSlippageWarningLevel } from "@/lib/slippage";

export type SlippageTier = "safe" | "low" | "high";

export interface SlippageAcknowledgmentState {
  tier: SlippageTier;
  /** Warning message for the current tier, or null if safe */
  message: string | null;
  /** Whether the user has explicitly acknowledged a high-tier warning */
  acknowledged: boolean;
  /** Whether the swap submit should be blocked (high tier + not acknowledged) */
  blocked: boolean;
  acknowledge: () => void;
}

const TIER_MESSAGES: Record<SlippageTier, string | null> = {
  safe: null,
  low: "Very low slippage may cause your swap to fail.",
  high: "High slippage — you may receive significantly less than expected. Please acknowledge to continue.",
};

export function useSlippageAcknowledgment(
  slippage: number | null,
  /** Reset key: when base/quote/amount changes, acknowledgment resets */
  resetKey?: string,
): SlippageAcknowledgmentState {
  const level = getSlippageWarningLevel(slippage);
  const tier: SlippageTier = level ?? "safe";

  const [acknowledged, setAcknowledged] = useState(false);

  // Reset acknowledgment when resetKey or tier changes
  const prevResetKey = useRef(resetKey);
  const prevTier = useRef(tier);

  useEffect(() => {
    if (resetKey !== prevResetKey.current || tier !== prevTier.current) {
      setAcknowledged(false);
      prevResetKey.current = resetKey;
      prevTier.current = tier;
    }
  }, [resetKey, tier]);

  const acknowledge = useCallback(() => setAcknowledged(true), []);

  const blocked = tier === "high" && !acknowledged;

  return {
    tier,
    message: TIER_MESSAGES[tier],
    acknowledged,
    blocked,
    acknowledge,
  };
}
