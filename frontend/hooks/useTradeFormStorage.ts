"use client";

import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "stellar-route-trade-form";
const DEFAULT_AMOUNT = "";
const DEFAULT_SLIPPAGE = 0.5;
const DEFAULT_DEADLINE = 30; // 30 minutes

interface PersistedForm {
  amount: string;
  slippage: number;
  deadline: number;
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

function saveToStorage(amount: string, slippage: number, deadline: number) {
  try {
    const data: PersistedForm = { amount, slippage, deadline, savedAt: Date.now() };
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
  deadline: number;
  setDeadline: (v: number) => void;
  /** Clears persisted state and resets to defaults */
  reset: () => void;
  /** True once localStorage has been read on the client */
  isHydrated: boolean;
}

/**
 * Persists trade form inputs (amount + slippage + deadline) to localStorage.
 *
 * Quote data is intentionally never persisted — only input fields are
 * restored, so no stale price information can be acted upon after reload.
 */
export function useTradeFormStorage(): UseTradeFormStorageResult {
  const [isHydrated, setIsHydrated] = useState(false);
  const [amount, setAmountState] = useState(DEFAULT_AMOUNT);
  const [slippage, setSlippageState] = useState(DEFAULT_SLIPPAGE);
  const [deadline, setDeadlineState] = useState(DEFAULT_DEADLINE);

    // Hydrate from localStorage once on mount (client-side only)
  useEffect(() => {
    const saved = loadFromStorage();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const updates: any = {};
    
    if (typeof saved.amount === "string") updates.amount = saved.amount;
    if (typeof saved.slippage === "number" && isFinite(saved.slippage)) updates.slippage = saved.slippage;
    if (typeof saved.deadline === "number" && isFinite(saved.deadline)) updates.deadline = saved.deadline;

    if (updates.amount !== undefined) setAmountState(updates.amount);
    if (updates.slippage !== undefined) setSlippageState(updates.slippage);
    if (updates.deadline !== undefined) setDeadlineState(updates.deadline);
    
    setIsHydrated(true);
  }, []);

  const setAmount = useCallback(
    (v: string) => {
      setAmountState(v);
      if (isHydrated) saveToStorage(v, slippage, deadline);
    },
    [isHydrated, slippage, deadline]
  );

  const setSlippage = useCallback(
    (v: number) => {
      setSlippageState(v);
      if (isHydrated) saveToStorage(amount, v, deadline);
    },
    [isHydrated, amount, deadline]
  );

  const setDeadline = useCallback(
    (v: number) => {
      setDeadlineState(v);
      if (isHydrated) saveToStorage(amount, slippage, v);
    },
    [isHydrated, amount, slippage]
  );

  const reset = useCallback(() => {
    setAmountState(DEFAULT_AMOUNT);
    setSlippageState(DEFAULT_SLIPPAGE);
    setDeadlineState(DEFAULT_DEADLINE);
    clearStorage();
  }, []);

  return { amount, setAmount, slippage, setSlippage, deadline, setDeadline, reset, isHydrated };
}
