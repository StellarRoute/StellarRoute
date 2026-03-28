"use client";

import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "stellar-route-trade-form";
const DEFAULT_AMOUNT = "";
const DEFAULT_SLIPPAGE = 0.5;

interface PersistedForm {
  amount: string;
  slippage: number;
  savedAt: number;
}

function loadFromStorage(): Partial<PersistedForm> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) return {};
    return parsed as Partial<PersistedForm>;
  } catch {
    return {};
  }
}

function saveToStorage(amount: string, slippage: number) {
  try {
    const data: PersistedForm = { amount, slippage, savedAt: Date.now() };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
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

export interface UseTradeFormStorageResult {
  amount: string;
  setAmount: (v: string) => void;
  slippage: number;
  setSlippage: (v: number) => void;
  /** Clears persisted state and resets to defaults */
  reset: () => void;
  /** True once localStorage has been read on the client */
  isHydrated: boolean;
}

/**
 * Persists trade form inputs (amount + slippage) to localStorage.
 *
 * Quote data is intentionally never persisted — only input fields are
 * restored, so no stale price information can be acted upon after reload.
 */
export function useTradeFormStorage(): UseTradeFormStorageResult {
  const [isHydrated, setIsHydrated] = useState(false);
  const [amount, setAmountState] = useState(DEFAULT_AMOUNT);
  const [slippage, setSlippageState] = useState(DEFAULT_SLIPPAGE);

  // Hydrate from localStorage once on mount (client-side only)
  useEffect(() => {
    const saved = loadFromStorage();
    if (typeof saved.amount === "string") {
      setAmountState(saved.amount);
    }
    if (typeof saved.slippage === "number" && isFinite(saved.slippage)) {
      setSlippageState(saved.slippage);
    }
    setIsHydrated(true);
  }, []);

  const setAmount = useCallback(
    (v: string) => {
      setAmountState(v);
      if (isHydrated) saveToStorage(v, slippage);
    },
    [isHydrated, slippage]
  );

  const setSlippage = useCallback(
    (v: number) => {
      setSlippageState(v);
      if (isHydrated) saveToStorage(amount, v);
    },
    [isHydrated, amount]
  );

  const reset = useCallback(() => {
    setAmountState(DEFAULT_AMOUNT);
    setSlippageState(DEFAULT_SLIPPAGE);
    clearStorage();
  }, []);

  return { amount, setAmount, slippage, setSlippage, reset, isHydrated };
}
