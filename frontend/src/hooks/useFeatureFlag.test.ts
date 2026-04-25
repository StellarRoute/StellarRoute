import { vi, describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import {
  useFeatureFlag,
  useFeatureFlags,
  invalidateFlagCache,
} from "./useFeatureFlag";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function mockFetch(flags: Record<string, boolean>) {
  global.fetch = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => flags,
  } as Response);
}

beforeEach(() => {
  invalidateFlagCache();
  delete process.env.NEXT_PUBLIC_FLAGS_URL;
  delete process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA;
  delete process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2;
});

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("useFeatureFlag", () => {
  it("defaults to false when no env or remote config", async () => {
    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(false);
  });

  it("reads flag from env var", async () => {
    process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA = "true";
    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(true);
  });

  it("remote config takes priority over env", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA = "false";
    mockFetch({ routes_beta: true });

    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(true);
  });

  it("falls back to false on remote fetch failure", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(false);
  });
});

describe("useFeatureFlags (batch)", () => {
  it("resolves multiple flags at once", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    mockFetch({ routes_beta: true, swap_ui_v2: false });

    const { result } = renderHook(() =>
      useFeatureFlags(["routes_beta", "swap_ui_v2"])
    );

    await waitFor(() => expect(result.current.routes_beta).toBe(true));
    expect(result.current.swap_ui_v2).toBe(false);
  });
});
