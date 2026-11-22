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
  usePairs: vi.fn(),
  usePriceHistory: vi.fn(),
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

const mockPairs = [
  {
    base: "XLM",
    counter: "USDC",
    base_asset: "native",
    counter_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    offer_count: 42,
  },
  {
    base: "XLM",
    counter: "EURC",
    base_asset: "native",
    counter_asset: "EURC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    offer_count: 17,
  },
];

const mockPriceHistory = {
  base_asset: { asset_type: "native" as const },
  quote_asset: {
    asset_type: "credit_alphanum4" as const,
    asset_code: "USDC",
    asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  },
  window: "24h" as const,
  source: "sdex",
  generated_at: 1_700_000_000_000,
  points: [
    { timestamp: 1_700_000_000_000, price: "0.1050000" },
    { timestamp: 1_700_003_600_000, price: "0.1062000" },
    { timestamp: 1_700_007_200_000, price: "0.1041000" },
  ],
};

/** Wire the two hooks the sparkline card depends on. */
function mockSparklineHooks({
  pairs = mockPairs,
  pairsLoading = false,
  history = mockPriceHistory,
  historyLoading = false,
}: {
  pairs?: typeof mockPairs | undefined;
  pairsLoading?: boolean;
  history?: typeof mockPriceHistory | undefined;
  historyLoading?: boolean;
} = {}) {
  vi.mocked(useApiHooks.usePairs).mockReturnValue({
    data: pairs,
    loading: pairsLoading,
    error: null,
    refresh: vi.fn(),
  } as unknown as ReturnType<typeof useApiHooks.usePairs>);
  vi.mocked(useApiHooks.usePriceHistory).mockReturnValue({
    data: history,
    loading: historyLoading,
    error: null,
    refresh: vi.fn(),
  } as unknown as ReturnType<typeof useApiHooks.usePriceHistory>);
}

describe("AnalyticsPageClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("shows placeholder when analytics feature flag is disabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });

    render(<AnalyticsPageClient />);

    expect(screen.getByText("Analytics preview disabled")).toBeInTheDocument();
    expect(screen.queryByText("Quote cache")).not.toBeInTheDocument();
  });

  it("renders live metrics when analytics feature flag is enabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: true, loading: false });
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
    mockSparklineHooks();

    render(<AnalyticsPageClient />);

    expect(screen.getByRole("heading", { name: "Analytics" })).toBeInTheDocument();
    expect(screen.getByText("Quote cache")).toBeInTheDocument();
    expect(screen.getByText("80.0%")).toBeInTheDocument();
    expect(screen.getByText("Primary pool")).toBeInTheDocument();
  });

  // ── 24h pair sparkline (issue #1260) ──────────────────────────────────

  function renderEnabled() {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: true, loading: false });
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
    return render(<AnalyticsPageClient />);
  }

  it("renders the 24h sparkline with mocked price history", () => {
    mockSparklineHooks();

    renderEnabled();

    expect(screen.getByLabelText("24 hour price sparkline")).toBeInTheDocument();
    expect(screen.getByText("XLM/USDC · 24h")).toBeInTheDocument();
    expect(screen.getByText("3 samples in the last 24h")).toBeInTheDocument();
  });

  it("offers a pair picker with the first pair selected by default", () => {
    mockSparklineHooks();

    renderEnabled();

    const picker = screen.getByRole("group", { name: "Select a trading pair" });
    expect(picker).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "XLM/USDC" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      screen.getByRole("button", { name: "XLM/EURC" }),
    ).toHaveAttribute("aria-pressed", "false");
  });

  it("shows the sparkline empty state when history has no points", () => {
    mockSparklineHooks({ history: { ...mockPriceHistory, points: [] } });

    renderEnabled();

    expect(
      screen.getByText("No 24h price data available yet."),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("24 hour price sparkline"),
    ).not.toBeInTheDocument();
  });

  it("shows an empty state when the indexer has no pairs", () => {
    mockSparklineHooks({ pairs: [] });

    renderEnabled();

    expect(screen.getByText("No markets available")).toBeInTheDocument();
  });

  it("does not render the sparkline when the analytics flag is off", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });
    mockSparklineHooks();

    render(<AnalyticsPageClient />);

    expect(
      screen.queryByLabelText("24 hour price sparkline"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("group", { name: "Select a trading pair" }),
    ).not.toBeInTheDocument();
  });

  // ── Skeleton loading state (issue #1732) ──────────────────────────────────

  it("renders the skeleton when the feature flag is loading", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: true });

    render(<AnalyticsPageClient />);

    const skeleton = screen.getByTestId("analytics-skeleton");
    expect(skeleton).toBeInTheDocument();
  });

  it("skeleton has aria-busy=true and an accessible label", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: true });

    render(<AnalyticsPageClient />);

    const skeleton = screen.getByTestId("analytics-skeleton");
    expect(skeleton).toHaveAttribute("aria-busy", "true");
    expect(skeleton).toHaveAttribute("aria-label", "Loading analytics");
  });

  it("does not show the disabled placeholder or the dashboard while loading", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: true });

    render(<AnalyticsPageClient />);

    expect(screen.queryByText("Analytics preview disabled")).not.toBeInTheDocument();
    expect(screen.queryByText("Quote cache")).not.toBeInTheDocument();
  });

  it("hides the skeleton once the flag resolves to disabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });

    render(<AnalyticsPageClient />);

    expect(screen.queryByTestId("analytics-skeleton")).not.toBeInTheDocument();
    expect(screen.getByText("Analytics preview disabled")).toBeInTheDocument();
  });

  it("hides the skeleton once the flag resolves to enabled and shows live metrics", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: true, loading: false });
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
    mockSparklineHooks();

    render(<AnalyticsPageClient />);

    expect(screen.queryByTestId("analytics-skeleton")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Analytics" })).toBeInTheDocument();
  });
});
