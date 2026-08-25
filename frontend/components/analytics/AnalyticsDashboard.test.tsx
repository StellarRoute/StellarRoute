import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AnalyticsDashboard } from "./AnalyticsDashboard";
import { useCacheMetrics, usePoolStats } from "@/hooks/useApi";

vi.mock("@/hooks/useApi", () => ({
  useCacheMetrics: vi.fn(),
  usePoolStats: vi.fn(),
}));

const mockUseCacheMetrics = vi.mocked(useCacheMetrics);
const mockUsePoolStats = vi.mocked(usePoolStats);

describe("AnalyticsDashboard", () => {
  beforeEach(() => {
    mockUseCacheMetrics.mockReset();
    mockUsePoolStats.mockReset();
  });

  it("shows a loading state while metrics are being fetched", () => {
    mockUseCacheMetrics.mockReturnValue({
      data: undefined,
      loading: true,
      error: null,
      refresh: vi.fn(),
    });
    mockUsePoolStats.mockReturnValue({
      data: undefined,
      loading: true,
      error: null,
      refresh: vi.fn(),
    });

    render(<AnalyticsDashboard />);

    expect(screen.getByText("Loading metrics")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh analytics metrics" })).toBeDisabled();
  });

  it("shows an error state when both metrics requests fail", () => {
    mockUseCacheMetrics.mockReturnValue({
      data: undefined,
      loading: false,
      error: new Error("cache unavailable"),
      refresh: vi.fn(),
    });
    mockUsePoolStats.mockReturnValue({
      data: undefined,
      loading: false,
      error: new Error("pool unavailable"),
      refresh: vi.fn(),
    });

    render(<AnalyticsDashboard />);

    expect(screen.getByText("Metrics unavailable")).toBeInTheDocument();
    expect(screen.getByText(/Could not load analytics data/)).toBeInTheDocument();
  });

  it("renders mocked metrics data", () => {
    mockUseCacheMetrics.mockReturnValue({
      data: {
        quote_hits: 120,
        quote_misses: 30,
        hit_ratio: 0.8,
        stale_quote_rejections: 2,
        stale_inputs_excluded: 5,
      },
      loading: false,
      error: null,
      refresh: vi.fn(),
    });
    mockUsePoolStats.mockReturnValue({
      data: {
        primary: {
          max_connections: 10,
          size: 6,
          idle: 2,
          in_use: 4,
          utilisation: 0.4,
        },
      },
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(<AnalyticsDashboard />);

    expect(screen.getByText("80.0%")).toBeInTheDocument();
    expect(screen.getByText("120")).toBeInTheDocument();
    expect(screen.getByText("Primary pool")).toBeInTheDocument();
  });
});
