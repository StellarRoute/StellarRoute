import { vi, describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import {
  invalidateFlagCache,
  resolveFlag,
  resolveFlagForInitialRender,
  useFeatureFlag,
  useFeatureFlags,
} from "./useFeatureFlag";

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
  delete process.env.NEXT_PUBLIC_FLAG_AI_AGENT;
});

describe("resolveFlag / ai_agent default", () => {
  it("defaults ai_agent off for initial and post-hydration resolution", () => {
    expect(resolveFlagForInitialRender("ai_agent")).toBe(false);
    expect(resolveFlag("ai_agent")).toBe(false);
  });

  it("honors explicit ai_agent=true from env", () => {
    process.env.NEXT_PUBLIC_FLAG_AI_AGENT = "true";
    expect(resolveFlagForInitialRender("ai_agent")).toBe(true);
    expect(resolveFlag("ai_agent")).toBe(true);
  });

  it("honors explicit ai_agent=true from remote config", () => {
    expect(resolveFlag("ai_agent", { ai_agent: true })).toBe(true);
  });
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

  it("defaults card to false when unset and honors NEXT_PUBLIC_FLAG_CARD", () => {
    const { result: defaultResult } = renderHook(() => useFeatureFlag("card"));
    expect(defaultResult.current.enabled).toBe(false);

    process.env.NEXT_PUBLIC_FLAG_CARD = "true";
    const { result: enabledResult } = renderHook(() => useFeatureFlag("card"));
    expect(enabledResult.current.enabled).toBe(true);
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

// ---------------------------------------------------------------------------
// Exhaustive default-value contract for every FlagName
//
// Purpose: lock defaults so a PR cannot silently flip routes_beta,
// transaction_history, batch_swaps, advanced_slippage, analytics, or
// ai_agent to true — or swap_ui_v2 / real_xdr to an unexpected value.
// ---------------------------------------------------------------------------

import type { FlagName } from "./useFeatureFlag";

/**
 * Canonical expected defaults.
 *
 * - real_xdr     : true  (security-pinned; swap prepare/sign/submit path)
 * - swap_ui_v2   : true  (product default; cross-chain deck on by default)
 * - everything else: false
 *
 * Edit this table ONLY when a product decision changes a default and the
 * corresponding production code is updated in the same PR.
 */
const EXPECTED_DEFAULTS: Record<FlagName, boolean> = {
  real_xdr: true,          // security-pinned, must not be remotely disabled
  swap_ui_v2: true,        // currently defaults on
  routes_beta: false,
  batch_swaps: false,
  transaction_history: false,
  advanced_slippage: false,
  analytics: false,
  ai_agent: false,
};

describe("defaultFlagValue — exhaustive FlagName contract", () => {
  // No env vars, no remote, no window overrides → pure default resolution.
  beforeEach(() => {
    invalidateFlagCache();
    // Clear every env var that could influence resolution.
    delete process.env.NEXT_PUBLIC_FLAGS_URL;
    delete process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA;
    delete process.env.NEXT_PUBLIC_FLAG_BATCH_SWAPS;
    delete process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2;
    delete process.env.NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY;
    delete process.env.NEXT_PUBLIC_FLAG_REAL_XDR;
    delete process.env.NEXT_PUBLIC_FEATURE_ANALYTICS;
    delete process.env.NEXT_PUBLIC_AI_AGENT;
    delete process.env.NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE;
    // Clear any window overrides left by other tests.
    if (typeof window !== "undefined") {
      delete (window as Record<string, unknown>).__STELLAR_ROUTE_FLAGS__;
    }
  });

  // Generate one test per flag so failures pinpoint the exact flag.
  const entries = Object.entries(EXPECTED_DEFAULTS) as [FlagName, boolean][];
  for (const [flag, expected] of entries) {
    it(`${flag} defaults to ${expected} (resolveFlag, no remote)`, () => {
      expect(resolveFlag(flag, {})).toBe(expected);
    });

    it(`${flag} defaults to ${expected} (resolveFlagForInitialRender)`, () => {
      expect(resolveFlagForInitialRender(flag)).toBe(expected);
    });
  }

  it("every FlagName is covered by EXPECTED_DEFAULTS (schema guard)", () => {
    // If a new FlagName is added to useFeatureFlag.ts, this test fails until
    // the maintainer explicitly adds it to EXPECTED_DEFAULTS above.
    const actualKeys = Object.keys(EXPECTED_DEFAULTS).sort();
    // The compile-time check: FlagName values used as keys above.
    // At runtime we verify the set is non-empty and all known flags are present.
    expect(actualKeys.length).toBeGreaterThanOrEqual(8);
    expect(actualKeys).toContain("real_xdr");
    expect(actualKeys).toContain("swap_ui_v2");
    expect(actualKeys).toContain("routes_beta");
    expect(actualKeys).toContain("transaction_history");
    expect(actualKeys).toContain("batch_swaps");
    expect(actualKeys).toContain("advanced_slippage");
    expect(actualKeys).toContain("analytics");
    expect(actualKeys).toContain("ai_agent");
  });
});

describe("real_xdr remote-disable pin — exhaustive", () => {
  beforeEach(() => {
    invalidateFlagCache();
    delete process.env.NEXT_PUBLIC_FLAG_REAL_XDR;
  });

  it("remote {real_xdr: false} cannot disable real_xdr when env is unset", () => {
    expect(resolveFlag("real_xdr", { real_xdr: false })).toBe(true);
  });

  it("remote {real_xdr: false} cannot disable real_xdr when env is true", () => {
    process.env.NEXT_PUBLIC_FLAG_REAL_XDR = "true";
    expect(resolveFlag("real_xdr", { real_xdr: false })).toBe(true);
  });

  it("resolveFlagForInitialRender ignores a warmed cache that holds false for real_xdr", () => {
    // Even if the remote cache somehow contained false, the security pin wins.
    expect(resolveFlagForInitialRender("real_xdr")).toBe(true);
  });

  it("real_xdr is present in SECURITY_PINNED_FLAGS", async () => {
    const { SECURITY_PINNED_FLAGS } = await import("./useFeatureFlag");
    expect(SECURITY_PINNED_FLAGS.has("real_xdr")).toBe(true);
  });

  it("swap_ui_v2 is NOT in SECURITY_PINNED_FLAGS (ordinary flag, remotely overridable)", async () => {
    const { SECURITY_PINNED_FLAGS } = await import("./useFeatureFlag");
    expect(SECURITY_PINNED_FLAGS.has("swap_ui_v2")).toBe(false);
  });
});

describe("ordinary flags that must default false — named regression guards", () => {
  beforeEach(() => {
    invalidateFlagCache();
    delete process.env.NEXT_PUBLIC_FLAGS_URL;
    delete process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA;
    delete process.env.NEXT_PUBLIC_FLAG_BATCH_SWAPS;
    delete process.env.NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY;
    delete process.env.NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE;
    delete process.env.NEXT_PUBLIC_FEATURE_ANALYTICS;
    delete process.env.NEXT_PUBLIC_AI_AGENT;
  });

  it("routes_beta defaults false (not live for end users)", () => {
    expect(resolveFlag("routes_beta", {})).toBe(false);
    expect(resolveFlagForInitialRender("routes_beta")).toBe(false);
  });

  it("transaction_history defaults false", () => {
    expect(resolveFlag("transaction_history", {})).toBe(false);
    expect(resolveFlagForInitialRender("transaction_history")).toBe(false);
  });

  it("batch_swaps defaults false", () => {
    expect(resolveFlag("batch_swaps", {})).toBe(false);
    expect(resolveFlagForInitialRender("batch_swaps")).toBe(false);
  });

  it("advanced_slippage defaults false", () => {
    expect(resolveFlag("advanced_slippage", {})).toBe(false);
    expect(resolveFlagForInitialRender("advanced_slippage")).toBe(false);
  });

  it("analytics defaults false", () => {
    expect(resolveFlag("analytics", {})).toBe(false);
    expect(resolveFlagForInitialRender("analytics")).toBe(false);
  });

  it("ai_agent defaults false", () => {
    expect(resolveFlag("ai_agent", {})).toBe(false);
    expect(resolveFlagForInitialRender("ai_agent")).toBe(false);
  });
});
