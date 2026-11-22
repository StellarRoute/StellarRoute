import { renderHook, act } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { useRecentPairs, type RecentPair } from "./useRecentPairs";

const STORAGE_KEY = "stellar-route-recent-pairs";

function makePair(base: string, quote: string): RecentPair {
  return { base, quote, baseAsset: `native-${base}`, quoteAsset: `asset-${quote}` };
}

describe("useRecentPairs", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("starts empty when localStorage has no entry", () => {
    const { result } = renderHook(() => useRecentPairs());
    expect(result.current.recentPairs).toHaveLength(0);
  });

  it("hydrates from localStorage on mount", () => {
    const stored: RecentPair[] = [makePair("XLM", "USDC")];
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));

    const { result } = renderHook(() => useRecentPairs());
    expect(result.current.recentPairs).toHaveLength(1);
    expect(result.current.recentPairs[0].base).toBe("XLM");
  });

  it("adds a pair to the front of the list", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
    });

    expect(result.current.recentPairs[0].base).toBe("XLM");
    expect(result.current.recentPairs[0].quote).toBe("USDC");
  });

  it("persists to localStorage when a pair is added", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
    });

    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
    expect(stored).toHaveLength(1);
    expect(stored[0].base).toBe("XLM");
  });

  it("caps the list at 5 entries (LRU eviction)", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
      result.current.addRecentPair(makePair("XLM", "BTC"));
      result.current.addRecentPair(makePair("XLM", "ETH"));
      result.current.addRecentPair(makePair("XLM", "EURC"));
      result.current.addRecentPair(makePair("XLM", "AQUA"));
      result.current.addRecentPair(makePair("XLM", "yXLM")); // 6th — should evict oldest
    });

    expect(result.current.recentPairs).toHaveLength(5);
    // Most recent is at front
    expect(result.current.recentPairs[0].quote).toBe("yXLM");
    // Oldest (USDC) is evicted
    expect(result.current.recentPairs.map((p) => p.quote)).not.toContain("USDC");
  });

  it("promotes a duplicate pair to position 0 (LRU semantics)", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
      result.current.addRecentPair(makePair("XLM", "BTC"));
      // Re-add USDC — should jump to front, not create a duplicate
      result.current.addRecentPair(makePair("XLM", "USDC"));
    });

    expect(result.current.recentPairs).toHaveLength(2);
    expect(result.current.recentPairs[0].quote).toBe("USDC");
    expect(result.current.recentPairs[1].quote).toBe("BTC");
  });

  it("does not add duplicate entries", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
      result.current.addRecentPair(makePair("XLM", "USDC"));
      result.current.addRecentPair(makePair("XLM", "USDC"));
    });

    expect(result.current.recentPairs).toHaveLength(1);
  });

  it("clears the list and removes localStorage entry", () => {
    const { result } = renderHook(() => useRecentPairs());

    act(() => {
      result.current.addRecentPair(makePair("XLM", "USDC"));
      result.current.addRecentPair(makePair("XLM", "BTC"));
    });

    expect(result.current.recentPairs).toHaveLength(2);

    act(() => {
      result.current.clearRecentPairs();
    });

    expect(result.current.recentPairs).toHaveLength(0);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("ignores corrupt localStorage data gracefully", () => {
    localStorage.setItem(STORAGE_KEY, "not-valid-json{{");
    const { result } = renderHook(() => useRecentPairs());
    expect(result.current.recentPairs).toHaveLength(0);
  });

  it("ignores localStorage data with wrong shape", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([{ foo: "bar" }]));
    const { result } = renderHook(() => useRecentPairs());
    expect(result.current.recentPairs).toHaveLength(0);
  });
});
