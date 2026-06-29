import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";

import { AnalyticsPageClient } from "./AnalyticsPageClient";
import * as useApiHooks from "@/hooks/useApi";

vi.mock("@/hooks/useFeatureFlag", () => ({
  useFeatureFlag: vi.fn(),
}));

vi.mock("@/hooks/useApi", () => ({
  useCacheMetrics: vi.fn(),
  usePoolStats: vi.fn(),
}));

import { useFeatureFlag } from "@/hooks/useFeatureFlag";

const mockCacheMetrics = {
  quote_hits: 120,
  quote_misses: 30,
  hit_ratio: 0.8,
  stale_quote_rejections: 2,
  stale_inputs_excluded: 5,
};

const mockPoolStats = {
  primary: {
    max_connections: 10,
    size: 6,
    idle: 2,
    in_use: 4,
    utilisation: 0.4,
  },
};

describe("AnalyticsPageClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("shows placeholder when analytics feature flag is disabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue(false);

    render(<AnalyticsPageClient />);

    expect(screen.getByText("Analytics preview disabled")).toBeInTheDocument();
    expect(screen.queryByText("Quote cache")).not.toBeInTheDocument();
  });

  it("renders live metrics when analytics feature flag is enabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue(true);
    vi.mocked(useApiHooks.useCacheMetrics).mockReturnValue({
      data: mockCacheMetrics,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });
    vi.mocked(useApiHooks.usePoolStats).mockReturnValue({
      data: mockPoolStats,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(<AnalyticsPageClient />);

    expect(screen.getByRole("heading", { name: "Analytics" })).toBeInTheDocument();
    expect(screen.getByText("Quote cache")).toBeInTheDocument();
    expect(screen.getByText("80.0%")).toBeInTheDocument();
    expect(screen.getByText("Primary pool")).toBeInTheDocument();
  });
});
