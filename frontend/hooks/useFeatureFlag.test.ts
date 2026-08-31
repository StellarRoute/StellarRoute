import { vi, describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import {
  invalidateFlagCache,
  resolveFlag,
  resolveFlagForInitialRender,
  useFeatureFlag,
  useFeatureFlags,
} from "./useFeatureFlag";
import type { FlagName } from "./useFeatureFlag";

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
  delete process.env.NEXT_PUBLIC_FLAG_REAL_XDR;
});

describe("resolveFlag / real_xdr security pin", () => {
  it("defaults real_xdr to true when unset", () => {
    expect(resolveFlag("real_xdr")).toBe(true);
  });

  it("honors explicit real_xdr=false from env", () => {
    process.env.NEXT_PUBLIC_FLAG_REAL_XDR = "false";
    expect(resolveFlag("real_xdr", { real_xdr: true })).toBe(false);
  });

  it("ignores remote false when env/default pins real_xdr on", () => {
    // Production-like: unset env → default true; remote cannot disable.
    expect(resolveFlag("real_xdr", { real_xdr: false })).toBe(true);

    process.env.NEXT_PUBLIC_FLAG_REAL_XDR = "true";
    expect(resolveFlag("real_xdr", { real_xdr: false })).toBe(true);
  });
});

describe("resolveFlag / swap_ui_v2 default", () => {
  it("defaults swap_ui_v2 on for initial and post-hydration resolution", () => {
    expect(resolveFlagForInitialRender("swap_ui_v2")).toBe(true);
    expect(resolveFlag("swap_ui_v2")).toBe(true);
  });

  it("honors explicit swap_ui_v2=false from env", () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = "false";
    expect(resolveFlagForInitialRender("swap_ui_v2")).toBe(false);
    expect(resolveFlag("swap_ui_v2")).toBe(false);
  });

  it("honors explicit swap_ui_v2=false from remote config", () => {
    expect(resolveFlag("swap_ui_v2", { swap_ui_v2: false })).toBe(false);
  });
});

describe("useFeatureFlag", () => {
  it("defaults to false when no env or remote config", async () => {
    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(false);
  });

  it("resolves swap_ui_v2 from env synchronously without loading flash", () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = "true";
    const { result } = renderHook(() => useFeatureFlag("swap_ui_v2"));
    expect(result.current.loading).toBe(false);
    expect(result.current.enabled).toBe(true);
  });

  it("waits for remote flags when only FLAGS_URL is configured", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    mockFetch({ swap_ui_v2: true });
    const { result } = renderHook(() => useFeatureFlag("swap_ui_v2"));
    expect(result.current.loading).toBe(true);
    expect(result.current.enabled).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(true);
  });

  it("defaults real_xdr to true when unset (server XDR product path)", async () => {
    const { result } = renderHook(() => useFeatureFlag("real_xdr"));
    // Security-pinned: resolves synchronously — no loading flash to false.
    expect(result.current.loading).toBe(false);
    expect(result.current.enabled).toBe(true);
  });

  it("honors explicit real_xdr=false", async () => {
    process.env.NEXT_PUBLIC_FLAG_REAL_XDR = "false";
    const { result } = renderHook(() => useFeatureFlag("real_xdr"));
    expect(result.current.loading).toBe(false);
    expect(result.current.enabled).toBe(false);
  });

  it("remote false + production env/default true → real_xdr remains enabled", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    process.env.NEXT_PUBLIC_FLAG_REAL_XDR = "true";
    mockFetch({ real_xdr: false, routes_beta: true });

    const { result } = renderHook(() => useFeatureFlag("real_xdr"));
    expect(result.current.enabled).toBe(true);
    expect(result.current.loading).toBe(false);

    // Ordinary flags still honor remote.
    const { result: routes } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(routes.current.loading).toBe(false));
    expect(routes.current.enabled).toBe(true);
  });

  it("remote false + default true (env unset) → real_xdr remains enabled", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    mockFetch({ real_xdr: false });

    const { result } = renderHook(() => useFeatureFlag("real_xdr"));
    expect(result.current.enabled).toBe(true);
    expect(result.current.loading).toBe(false);
  });

  it("reads flag from env var", async () => {
    process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA = "true";
    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.enabled).toBe(true);
  });

  it("remote config takes priority over env for ordinary flags", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA = "false";
    mockFetch({ routes_beta: true });

    const { result } = renderHook(() => useFeatureFlag("routes_beta"));
    await waitFor(() => expect(result.current.enabled).toBe(true));
  });

  it("window override beats env after mount when remote is absent", async () => {
    process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2 = "true";
    expect(resolveFlagForInitialRender("swap_ui_v2")).toBe(true);

    (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ = {
      swap_ui_v2: false,
    };

    const { result } = renderHook(() => useFeatureFlag("swap_ui_v2"));
    await waitFor(() => expect(result.current.enabled).toBe(false));
    expect(result.current.loading).toBe(false);
  });

  it("remote beats window override after fetch", async () => {
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example.com/flags.json";
    (window as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> }).__STELLAR_ROUTE_FLAGS__ = {
      swap_ui_v2: false,
    };
    mockFetch({ swap_ui_v2: true });

    const { result } = renderHook(() => useFeatureFlag("swap_ui_v2"));
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

describe("ordinary feature flag defaults", () => {
  const ALL_FLAGS: FlagName[] = [
    "routes_beta",
    "batch_swaps",
    "swap_ui_v2",
    "transaction_history",
    "advanced_slippage",
    "real_xdr",
    "analytics",
  ];

  const EXPECTED_DEFAULTS: Record<FlagName, boolean> = {
    routes_beta: false,
    batch_swaps: false,
    swap_ui_v2: true,
    transaction_history: false,
    advanced_slippage: false,
    real_xdr: true,
    analytics: false,
  };

  beforeEach(() => {
    delete process.env.NEXT_PUBLIC_FLAG_BATCH_SWAPS;
    delete process.env.NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY;
    delete process.env.NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE;
    delete process.env.NEXT_PUBLIC_FEATURE_ANALYTICS;
    delete (window as { __STELLAR_ROUTE_FLAGS__?: unknown })
      .__STELLAR_ROUTE_FLAGS__;
  });

  it.each(ALL_FLAGS)("resolves %s to its documented default", (flag) => {
    expect(resolveFlag(flag)).toBe(EXPECTED_DEFAULTS[flag]);
  });

  it("keeps every ordinary flag except swap_ui_v2 off by default", () => {
    const on = ALL_FLAGS.filter(
      (flag) => flag !== "real_xdr" && resolveFlag(flag) === true,
    );
    expect(on).toEqual(["swap_ui_v2"]);
  });

  it("keeps real_xdr pinned on even when remote JSON disables it", async () => {
    mockFetch({ real_xdr: false });
    process.env.NEXT_PUBLIC_FLAGS_URL = "https://flags.example/flags.json";

    const { result } = renderHook(() => useFeatureFlag("real_xdr"));
    await waitFor(() => expect(result.current.enabled).toBe(true));
  });
});
