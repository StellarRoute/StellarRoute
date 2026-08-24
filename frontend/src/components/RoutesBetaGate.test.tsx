import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { RoutesBetaGate } from "./RoutesBetaGate";

vi.mock("@/hooks/useFeatureFlag", () => ({
  useFeatureFlag: vi.fn(),
}));

import { useFeatureFlag } from "@/hooks/useFeatureFlag";

describe("RoutesBetaGate", () => {
  beforeEach(() => {
    vi.mocked(useFeatureFlag).mockReset();
  });

  it("renders children when routesBeta is enabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: true, loading: false });

    render(
      <RoutesBetaGate fallback={<div data-testid="fallback">legacy</div>}>
        <div data-testid="beta">beta routes</div>
      </RoutesBetaGate>
    );

    expect(screen.getByTestId("beta")).toBeInTheDocument();
    expect(screen.queryByTestId("fallback")).not.toBeInTheDocument();
  });

  it("renders fallback when routesBeta is disabled", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });

    render(
      <RoutesBetaGate fallback={<div data-testid="fallback">legacy</div>}>
        <div data-testid="beta">beta routes</div>
      </RoutesBetaGate>
    );

    expect(screen.getByTestId("fallback")).toBeInTheDocument();
    expect(screen.queryByTestId("beta")).not.toBeInTheDocument();
  });
});
