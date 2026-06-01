import { describe, expect, it } from "vitest";

import {
  getFeatureFlag,
  getFeatureFlags,
} from "@/lib/feature-flags";

describe("feature flags", () => {
  it("defaults risky flags to off", () => {
    delete (window as Window & { __STELLAR_ROUTE_FLAGS__?: unknown })
      .__STELLAR_ROUTE_FLAGS__;
    delete process.env.NEXT_PUBLIC_FEATURE_ROUTES_BETA;
    delete process.env.NEXT_PUBLIC_FEATURE_BATCH_SWAPS;

    expect(getFeatureFlag("routesBeta")).toBe(false);
    expect(getFeatureFlag("batchSwaps")).toBe(false);
  });

  it("reads env-backed defaults when enabled", () => {
    process.env.NEXT_PUBLIC_FEATURE_ROUTES_BETA = "true";
    process.env.NEXT_PUBLIC_FEATURE_BATCH_SWAPS = "true";

    expect(getFeatureFlags().routesBeta).toBe(true);
    expect(getFeatureFlags().batchSwaps).toBe(true);
  });

  it("lets runtime config override env defaults", () => {
    process.env.NEXT_PUBLIC_FEATURE_ROUTES_BETA = "false";
    process.env.NEXT_PUBLIC_FEATURE_BATCH_SWAPS = "false";
    (
      window as Window & {
        __STELLAR_ROUTE_FLAGS__?: { routesBeta?: boolean; batchSwaps?: boolean };
      }
    ).__STELLAR_ROUTE_FLAGS__ = { routesBeta: true, batchSwaps: true };

    expect(getFeatureFlag("routesBeta")).toBe(true);
    expect(getFeatureFlag("batchSwaps")).toBe(true);
  });
});
