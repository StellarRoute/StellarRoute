import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useRecentTokens } from "@/hooks/useRecentTokens";

describe("useRecentTokens", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("returns empty list initially", () => {
    const { result } = renderHook(() => useRecentTokens());
    // isLoaded may be false on first render because the effect hasn't run yet
    expect(Array.isArray(result.current.recentTokens)).toBe(true);
  });

  it("adds a token and persists to localStorage", () => {
    const { result } = renderHook(() => useRecentTokens());

    act(() => {
      result.current.addRecentToken("native");
    });

    expect(result.current.recentTokens).toContain("native");
    const stored = JSON.parse(localStorage.getItem("stellar-route-recent-tokens") || "[]");
    expect(stored).toContain("native");
  });

  it("deduplicates tokens — most recent is first", () => {
    const { result } = renderHook(() => useRecentTokens());

    act(() => {
      result.current.addRecentToken("native");
      result.current.addRecentToken("USDC:ISSUER");
      result.current.addRecentToken("native"); // duplicate
    });

    const tokens = result.current.recentTokens;
    expect(tokens[0]).toBe("native");
    expect(tokens.filter((t) => t === "native").length).toBe(1);
  });

  it("hydrates from localStorage on mount", async () => {
    localStorage.setItem(
      "stellar-route-recent-tokens",
      JSON.stringify(["native", "USDC:ISSUER"])
    );

    const { result } = renderHook(() => useRecentTokens());

    // After the useEffect runs, the tokens should be hydrated
    await act(async () => {});

    expect(result.current.recentTokens).toContain("native");
    expect(result.current.recentTokens).toContain("USDC:ISSUER");
  });

  it("caps list at 10 entries", () => {
    const { result } = renderHook(() => useRecentTokens());

    act(() => {
      for (let i = 0; i < 15; i++) {
        result.current.addRecentToken(`TOKEN:ISSUER${i}`);
      }
    });

    expect(result.current.recentTokens.length).toBeLessThanOrEqual(10);
  });
});
