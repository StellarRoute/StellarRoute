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

    expect(getFeatureFlag("routesBeta")).toBe(false);
  });

  it("reads env-backed defaults when enabled", () => {
    process.env.NEXT_PUBLIC_FEATURE_ROUTES_BETA = "true";

    expect(getFeatureFlags().routesBeta).toBe(true);
  });

  it("lets runtime config override env defaults", () => {
    process.env.NEXT_PUBLIC_FEATURE_ROUTES_BETA = "false";
    (
      window as Window & {
        __STELLAR_ROUTE_FLAGS__?: { routesBeta?: boolean };
      }
    ).__STELLAR_ROUTE_FLAGS__ = { routesBeta: true };

    expect(getFeatureFlag("routesBeta")).toBe(true);
  });
});
