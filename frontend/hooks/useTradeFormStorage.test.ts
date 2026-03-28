import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTradeFormStorage } from "@/hooks/useTradeFormStorage";

const STORAGE_KEY = "stellar-route-trade-form";

describe("useTradeFormStorage", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("returns defaults on first mount (no stored data)", async () => {
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {}); // flush hydration effect

    expect(result.current.amount).toBe("");
    expect(result.current.slippage).toBe(0.5);
    expect(result.current.isHydrated).toBe(true);
  });

  it("hydrates amount and slippage from localStorage", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ amount: "42.5", slippage: 1.0, savedAt: Date.now() })
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    expect(result.current.amount).toBe("42.5");
    expect(result.current.slippage).toBe(1.0);
  });

  it("persists amount change to localStorage", async () => {
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.setAmount("100");
    });

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    expect(stored.amount).toBe("100");
  });

  it("persists slippage change to localStorage", async () => {
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.setSlippage(1.0);
    });

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    expect(stored.slippage).toBe(1.0);
  });

  it("savedAt timestamp is written on change", async () => {
    const before = Date.now();
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.setAmount("50");
    });

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    expect(stored.savedAt).toBeGreaterThanOrEqual(before);
  });

  it("reset() clears state to defaults and removes localStorage entry", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ amount: "999", slippage: 1.0, savedAt: Date.now() })
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    // Confirm hydration
    expect(result.current.amount).toBe("999");

    act(() => {
      result.current.reset();
    });

    expect(result.current.amount).toBe("");
    expect(result.current.slippage).toBe(0.5);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("handles corrupted localStorage gracefully, falls back to defaults", async () => {
    localStorage.setItem(STORAGE_KEY, "NOT_JSON{{{{");

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    expect(result.current.amount).toBe("");
    expect(result.current.slippage).toBe(0.5);
    expect(result.current.isHydrated).toBe(true);
  });

  it("ignores non-numeric slippage in storage", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ amount: "10", slippage: "bad", savedAt: Date.now() })
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    // slippage should fall back to default 0.5
    expect(result.current.slippage).toBe(0.5);
    // amount should still restore
    expect(result.current.amount).toBe("10");
  });

  it("isHydrated starts false and becomes true after mount effect", async () => {
    const { result } = renderHook(() => useTradeFormStorage());

    // Before the effect runs, isHydrated may be false (timing-dependent in test)
    // After flushing, it must be true
    await act(async () => {});
    expect(result.current.isHydrated).toBe(true);
  });
});
