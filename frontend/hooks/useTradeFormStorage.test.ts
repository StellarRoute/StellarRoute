import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  DEFAULT_TO_TOKEN,
  STORAGE_KEY,
  useTradeFormStorage,
} from "@/hooks/useTradeFormStorage";

describe("useTradeFormStorage", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("returns defaults on first mount with no pending recovery", async () => {
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    expect(result.current.amount).toBe("");
    expect(result.current.slippage).toBe(0.5);
    expect(result.current.deadline).toBe(30);
    expect(result.current.fromToken).toBe("native");
    expect(result.current.toToken).toBe(DEFAULT_TO_TOKEN);
    expect(result.current.pendingRecovery).toBeNull();
    expect(result.current.isHydrated).toBe(true);
  });

  it("stages stored context for explicit recovery instead of auto-hydrating", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: "42.5",
        slippage: 1.0,
        deadline: 45,
        fromToken: "native",
        toToken: "EURC:GEXAMPLE",
        savedAt: Date.now(),
      }),
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    expect(result.current.amount).toBe("");
    expect(result.current.pendingRecovery?.amount).toBe("42.5");
    expect(result.current.pendingRecovery?.toToken).toBe("EURC:GEXAMPLE");
  });

  it("restores the pending draft when requested", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: "42.5",
        slippage: 1.0,
        deadline: 45,
        fromToken: "native",
        toToken: "EURC:GEXAMPLE",
        savedAt: Date.now(),
      }),
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.restorePending();
    });

    expect(result.current.amount).toBe("42.5");
    expect(result.current.slippage).toBe(1.0);
    expect(result.current.deadline).toBe(45);
    expect(result.current.toToken).toBe("EURC:GEXAMPLE");
    expect(result.current.pendingRecovery).toBeNull();
  });

  it("discards pending recovery and clears persisted storage", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: "42.5",
        slippage: 1.0,
        deadline: 45,
        fromToken: "native",
        toToken: "EURC:GEXAMPLE",
        savedAt: Date.now(),
      }),
    );

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.discardPending();
    });

    expect(result.current.pendingRecovery).toBeNull();
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("persists token pair and amount changes to localStorage", async () => {
    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    act(() => {
      result.current.setTokenPair("EURC:GEXAMPLE", "native");
      result.current.setAmount("100");
    });

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    expect(stored.amount).toBe("100");
    expect(stored.fromToken).toBe("EURC:GEXAMPLE");
    expect(stored.toToken).toBe("native");
  });

  it("handles corrupted storage gracefully", async () => {
    localStorage.setItem(STORAGE_KEY, "NOT_JSON{{{{");

    const { result } = renderHook(() => useTradeFormStorage());
    await act(async () => {});

    expect(result.current.pendingRecovery).toBeNull();
    expect(result.current.amount).toBe("");
    expect(result.current.isHydrated).toBe(true);
  });
});
