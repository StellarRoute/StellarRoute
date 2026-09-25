import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";

import { CardPageClient } from "./CardPageClient";
import { useFeatureFlag } from "@/hooks/useFeatureFlag";

vi.mock("@/hooks/useFeatureFlag", () => ({
  useFeatureFlag: vi.fn(),
}));

describe("CardPageClient", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it("shows disabled preview state when card feature flag is off", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: false, loading: false });

    render(<CardPageClient />);

    expect(screen.getByText("Card program preview disabled")).toBeInTheDocument();
    expect(screen.queryByTestId("card-shell")).not.toBeInTheDocument();
  });

  it("renders empty card shell when card feature flag is on", () => {
    vi.mocked(useFeatureFlag).mockReturnValue({ enabled: true, loading: false });

    render(<CardPageClient />);

    expect(screen.getByTestId("card-shell")).toBeInTheDocument();
    expect(screen.getByText("StellarRoute Card")).toBeInTheDocument();
    expect(screen.getByText("Card Shell")).toBeInTheDocument();
  });
});
