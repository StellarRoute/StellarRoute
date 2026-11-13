"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { QuoteType } from "@/types";
import { readJson } from "@/lib/storage/safe-json";
import { STAGING_DEMO_ISSUER } from "@/lib/trading-pairs";

export const STORAGE_KEY = "stellar-route-trade-form";
export const DEFAULT_AMOUNT = "";
export const DEFAULT_SLIPPAGE = 0.5;
export const DEFAULT_DEADLINE = 30;

/** Prefer a staged demo market that currently quotes successfully (not native/XLM). */
export const DEFAULT_FROM_TOKEN = `EUR:${STAGING_DEMO_ISSUER}`;
/** Staging USDy — Circle USDC is not on the staging orderbook. */
export const DEFAULT_TO_TOKEN = `USDy:${STAGING_DEMO_ISSUER}`;
/** Prior Circle-USDC defaults — migrate away when indexed pairs load. */
export const LEGACY_DEFAULT_FROM_TOKEN = "native";
export const LEGACY_DEFAULT_TO_TOKEN =
  "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
export const DEFAULT_SIDE: QuoteType = "sell";
export const SESSION_RECOVERY_THRESHOLD_MS = 60_000;

export interface TradeFormSnapshot {
  amount: string;
  slippage: number;
  deadline: number;
  fromToken: string;
  toToken: string;
  side: QuoteType;
  savedAt: number;
}

type StoredTradeFormSnapshot = Omit<TradeFormSnapshot, "side" | "savedAt"> & {
  side?: QuoteType;
  savedAt?: number;
};

export function isTradeFormSnapshot(
  v: unknown,
): v is StoredTradeFormSnapshot {
  if (!v || typeof v !== "object" || Array.isArray(v)) return false;
  const candidate = v as Record<string, unknown>;
  if (typeof candidate.amount !== "string") return false;
  if (
    typeof candidate.slippage !== "number" ||
    !Number.isFinite(candidate.slippage)
  ) {
    return false;
  }
  if (
    typeof candidate.deadline !== "number" ||
    !Number.isFinite(candidate.deadline)
  ) {
    return false;
  }
  if (
    typeof candidate.fromToken !== "string" ||
    candidate.fromToken.length === 0
  ) {
    return false;
  }
  if (typeof candidate.toToken !== "string" || candidate.toToken.length === 0) {
    return false;
  }
  if (
    candidate.side !== undefined &&
    candidate.side !== "sell" &&
    candidate.side !== "buy"
  ) {
    return false;
  }
  if (
    candidate.savedAt !== undefined &&
    (typeof candidate.savedAt !== "number" ||
      !Number.isFinite(candidate.savedAt))
  ) {
    return false;
  }
  return true;
}

function loadSnapshot(): TradeFormSnapshot | null {
  const parsed = readJson(STORAGE_KEY, isTradeFormSnapshot, null);
  if (!parsed) return null;

  return {
    amount: parsed.amount,
    slippage: parsed.slippage,
    deadline: parsed.deadline,
    fromToken: parsed.fromToken,
    toToken: parsed.toToken,
    side: parsed.side ?? DEFAULT_SIDE,
    savedAt: parsed.savedAt ?? Date.now(),
  };
}

function saveSnapshot(snapshot: TradeFormSnapshot) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
  } catch {
    // quota exceeded or private browsing — silently ignore
  }
}

function clearStorage() {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}

function isRecoverableSnapshot(snapshot: TradeFormSnapshot | null): boolean {
  if (!snapshot) return false;

  return (
    snapshot.amount.trim().length > 0 ||
    snapshot.slippage !== DEFAULT_SLIPPAGE ||
    snapshot.deadline !== DEFAULT_DEADLINE ||
    snapshot.fromToken !== DEFAULT_FROM_TOKEN ||
    snapshot.toToken !== DEFAULT_TO_TOKEN ||
    snapshot.side !== DEFAULT_SIDE
  );
}

function buildSnapshot(
  amount: string,
  slippage: number,
  deadline: number,
  fromToken: string,
  toToken: string,
  side: QuoteType,
): TradeFormSnapshot {
  return {
    amount,
    slippage,
    deadline,
    fromToken,
    toToken,
    side,
    savedAt: Date.now(),
  };
}

export interface UseTradeFormStorageResult {
  amount: string;
  setAmount: (value: string) => void;
  slippage: number;
  setSlippage: (value: number) => void;
  deadline: number;
  setDeadline: (value: number) => void;
  fromToken: string;
  setFromToken: (value: string) => void;
  toToken: string;
  setToToken: (value: string) => void;
  side: QuoteType;
  setSide: (value: QuoteType) => void;
  setTokenPair: (nextFromToken: string, nextToToken: string) => void;
  pendingRecovery: TradeFormSnapshot | null;
  restorePending: () => void;
  discardPending: () => void;
  hasRecoverableState: boolean;
  snapshotCurrent: () => TradeFormSnapshot | null;
  reset: () => void;
  isHydrated: boolean;
}

/**
 * Persists non-sensitive trade form context while requiring explicit recovery
 * after a refresh. Quotes are intentionally excluded from storage.
 */
export function useTradeFormStorage(): UseTradeFormStorageResult {
  const [isHydrated, setIsHydrated] = useState(false);
  const [amount, setAmountState] = useState(DEFAULT_AMOUNT);
  const [slippage, setSlippageState] = useState(DEFAULT_SLIPPAGE);
  const [deadline, setDeadlineState] = useState(DEFAULT_DEADLINE);
  const [fromToken, setFromTokenState] = useState(DEFAULT_FROM_TOKEN);
  const [toToken, setToTokenState] = useState(DEFAULT_TO_TOKEN);
  const [side, setSideState] = useState<QuoteType>(DEFAULT_SIDE);
  const [pendingRecovery, setPendingRecovery] =
    useState<TradeFormSnapshot | null>(null);
  const stateRef = useRef({
    amount: DEFAULT_AMOUNT,
    slippage: DEFAULT_SLIPPAGE,
    deadline: DEFAULT_DEADLINE,
    fromToken: DEFAULT_FROM_TOKEN,
    toToken: DEFAULT_TO_TOKEN,
    side: DEFAULT_SIDE,
  });

  useEffect(() => {
    const snapshot = loadSnapshot();
    queueMicrotask(() => {
      if (isRecoverableSnapshot(snapshot)) {
        setPendingRecovery(snapshot);
      }
      setIsHydrated(true);
    });
  }, []);

  const persist = useCallback(
    (
      nextAmount: string,
      nextSlippage: number,
      nextDeadline: number,
      nextFromToken: string,
      nextToToken: string,
      nextSide: QuoteType,
    ) => {
      const snapshot = buildSnapshot(
        nextAmount,
        nextSlippage,
        nextDeadline,
        nextFromToken,
        nextToToken,
        nextSide,
      );
      saveSnapshot(snapshot);
    },
    [],
  );

  const setAmount = useCallback(
    (value: string) => {
      setAmountState(value);
      stateRef.current.amount = value;
      if (isHydrated) {
        persist(
          value,
          stateRef.current.slippage,
          stateRef.current.deadline,
          stateRef.current.fromToken,
          stateRef.current.toToken,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const setSlippage = useCallback(
    (value: number) => {
      setSlippageState(value);
      stateRef.current.slippage = value;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          value,
          stateRef.current.deadline,
          stateRef.current.fromToken,
          stateRef.current.toToken,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const setDeadline = useCallback(
    (value: number) => {
      setDeadlineState(value);
      stateRef.current.deadline = value;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          stateRef.current.slippage,
          value,
          stateRef.current.fromToken,
          stateRef.current.toToken,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const setFromToken = useCallback(
    (value: string) => {
      setFromTokenState(value);
      stateRef.current.fromToken = value;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          stateRef.current.slippage,
          stateRef.current.deadline,
          value,
          stateRef.current.toToken,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const setToToken = useCallback(
    (value: string) => {
      setToTokenState(value);
      stateRef.current.toToken = value;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          stateRef.current.slippage,
          stateRef.current.deadline,
          stateRef.current.fromToken,
          value,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const setSide = useCallback(
    (value: QuoteType) => {
      setSideState(value);
      stateRef.current.side = value;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          stateRef.current.slippage,
          stateRef.current.deadline,
          stateRef.current.fromToken,
          stateRef.current.toToken,
          value,
        );
      }
    },
    [isHydrated, persist],
  );

  const setTokenPair = useCallback(
    (nextFromToken: string, nextToToken: string) => {
      setFromTokenState(nextFromToken);
      setToTokenState(nextToToken);
      stateRef.current.fromToken = nextFromToken;
      stateRef.current.toToken = nextToToken;
      if (isHydrated) {
        persist(
          stateRef.current.amount,
          stateRef.current.slippage,
          stateRef.current.deadline,
          nextFromToken,
          nextToToken,
          stateRef.current.side,
        );
      }
    },
    [isHydrated, persist],
  );

  const restorePending = useCallback(() => {
    if (!pendingRecovery) return;

    setAmountState(pendingRecovery.amount);
    setSlippageState(pendingRecovery.slippage);
    setDeadlineState(pendingRecovery.deadline);
    setFromTokenState(pendingRecovery.fromToken);
    setToTokenState(pendingRecovery.toToken);
    setSideState(pendingRecovery.side);
    stateRef.current = {
      amount: pendingRecovery.amount,
      slippage: pendingRecovery.slippage,
      deadline: pendingRecovery.deadline,
      fromToken: pendingRecovery.fromToken,
      toToken: pendingRecovery.toToken,
      side: pendingRecovery.side,
    };
    persist(
      pendingRecovery.amount,
      pendingRecovery.slippage,
      pendingRecovery.deadline,
      pendingRecovery.fromToken,
      pendingRecovery.toToken,
      pendingRecovery.side,
    );
    setPendingRecovery(null);
  }, [pendingRecovery, persist]);

  const discardPending = useCallback(() => {
    setPendingRecovery(null);
    clearStorage();
  }, []);

  const reset = useCallback(() => {
    setAmountState(DEFAULT_AMOUNT);
    setSlippageState(DEFAULT_SLIPPAGE);
    setDeadlineState(DEFAULT_DEADLINE);
    setFromTokenState(DEFAULT_FROM_TOKEN);
    setToTokenState(DEFAULT_TO_TOKEN);
    setSideState(DEFAULT_SIDE);
    stateRef.current = {
      amount: DEFAULT_AMOUNT,
      slippage: DEFAULT_SLIPPAGE,
      deadline: DEFAULT_DEADLINE,
      fromToken: DEFAULT_FROM_TOKEN,
      toToken: DEFAULT_TO_TOKEN,
      side: DEFAULT_SIDE,
    };
    setPendingRecovery(null);
    clearStorage();
  }, []);

  const snapshotCurrent = useCallback(() => {
    const snapshot = buildSnapshot(amount, slippage, deadline, fromToken, toToken, side);
    return isRecoverableSnapshot(snapshot) ? snapshot : null;
  }, [amount, deadline, fromToken, slippage, toToken, side]);

  return {
    amount,
    setAmount,
    slippage,
    setSlippage,
    deadline,
    setDeadline,
    fromToken,
    setFromToken,
    toToken,
    setToToken,
    side,
    setSide,
    setTokenPair,
    pendingRecovery,
    restorePending,
    discardPending,
    hasRecoverableState: snapshotCurrent() !== null,
    snapshotCurrent,
    reset,
    isHydrated,
  };
}
