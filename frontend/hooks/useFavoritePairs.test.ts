import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useFavoritePairs } from "@/hooks/useFavoritePairs";

const PAIR_A = { base: "XLM", quote: "USDC", baseAsset: "native", quoteAsset: "USDC:ISSUER" };
const PAIR_B = { base: "XLM", quote: "BTC", baseAsset: "native", quoteAsset: "BTC:ISSUER" };

describe("useFavoritePairs", () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => localStorage.clear());

  it("starts empty", () => {
    const { result } = renderHook(() => useFavoritePairs());
    expect(result.current.favorites).toHaveLength(0);
  });

  it("adds a favorite", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => result.current.addFavorite(PAIR_A));
    expect(result.current.favorites).toHaveLength(1);
    expect(result.current.isFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset)).toBe(true);
  });

  it("does not duplicate favorites", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => {
      result.current.addFavorite(PAIR_A);
      result.current.addFavorite(PAIR_A);
    });
    expect(result.current.favorites).toHaveLength(1);
  });

  it("removes a favorite", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => result.current.addFavorite(PAIR_A));
    act(() => result.current.removeFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset));
    expect(result.current.favorites).toHaveLength(0);
    expect(result.current.isFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset)).toBe(false);
  });

  it("toggleFavorite adds when not present", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => result.current.toggleFavorite(PAIR_A));
    expect(result.current.isFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset)).toBe(true);
  });

  it("toggleFavorite removes when already present", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => result.current.addFavorite(PAIR_A));
    act(() => result.current.toggleFavorite(PAIR_A));
    expect(result.current.isFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset)).toBe(false);
  });

  it("persists to localStorage", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => result.current.addFavorite(PAIR_A));
    const stored = JSON.parse(localStorage.getItem("stellar-route-favorite-pairs") ?? "[]");
    expect(stored).toHaveLength(1);
    expect(stored[0].baseAsset).toBe(PAIR_A.baseAsset);
  });

  it("hydrates from localStorage on mount", () => {
    localStorage.setItem("stellar-route-favorite-pairs", JSON.stringify([PAIR_A, PAIR_B]));
    const { result } = renderHook(() => useFavoritePairs());
    expect(result.current.favorites).toHaveLength(2);
    expect(result.current.isFavorite(PAIR_A.baseAsset, PAIR_A.quoteAsset)).toBe(true);
  });

  it("isFavorite returns false for unknown pair", () => {
    const { result } = renderHook(() => useFavoritePairs());
    expect(result.current.isFavorite("native", "UNKNOWN:ISSUER")).toBe(false);
  });

  it("caps at 20 favorites", () => {
    const { result } = renderHook(() => useFavoritePairs());
    act(() => {
      for (let i = 0; i < 25; i++) {
        result.current.addFavorite({ base: "XLM", quote: `T${i}`, baseAsset: "native", quoteAsset: `T${i}:ISSUER` });
      }
    });
    expect(result.current.favorites.length).toBeLessThanOrEqual(20);
  });
});
