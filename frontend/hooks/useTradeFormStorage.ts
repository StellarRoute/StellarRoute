"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { QuoteType } from "@/types";

export const STORAGE_KEY = "stellar-route-trade-form";
export const DEFAULT_AMOUNT = "";
export const DEFAULT_SLIPPAGE = 0.5;
export const DEFAULT_DEADLINE = 30;
export const DEFAULT_FROM_TOKEN = "native";
export const DEFAULT_TO_TOKEN =
  "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W";
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

function parseSnapshot(raw: string | null): TradeFormSnapshot | null {
  if (!raw) return null;

  try {
    const parsed = JSON.parse(raw) as Partial<TradeFormSnapshot> | null;
    if (!parsed || typeof parsed !== "object") return null;

    const amount =
      typeof parsed.amount === "string" ? parsed.amount : DEFAULT_AMOUNT;
    const slippage =
      typeof parsed.slippage === "number" && Number.isFinite(parsed.slippage)
        ? parsed.slippage
        : DEFAULT_SLIPPAGE;
    const deadline =
      typeof parsed.deadline === "number" && Number.isFinite(parsed.deadline)
        ? parsed.deadline
        : DEFAULT_DEADLINE;
    const fromToken =
      typeof parsed.fromToken === "string" && parsed.fromToken.length > 0
        ? parsed.fromToken
        : DEFAULT_FROM_TOKEN;
    const toToken =
      typeof parsed.toToken === "string" && parsed.toToken.length > 0
        ? parsed.toToken
        : DEFAULT_TO_TOKEN;
    const side =
      parsed.side === "sell" || parsed.side === "buy"
        ? parsed.side
        : DEFAULT_SIDE;
    const savedAt =
      typeof parsed.savedAt === "number" && Number.isFinite(parsed.savedAt)
        ? parsed.savedAt
        : Date.now();

    return {
      amount,
      slippage,
      deadline,
      fromToken,
      toToken,
      side,
      savedAt,
    };
  } catch {
    return null;
  }
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
    const snapshot = parseSnapshot(localStorage.getItem(STORAGE_KEY));
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
