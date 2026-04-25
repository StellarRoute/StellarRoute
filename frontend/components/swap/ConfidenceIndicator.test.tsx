import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { ConfidenceIndicator } from "./ConfidenceIndicator";

describe("ConfidenceIndicator", () => {
  afterEach(() => cleanup());

  it("uses semantic theme token classes for confidence badges", () => {
    const { rerender } = render(<ConfidenceIndicator score={85} />);
    expect(screen.getByText("High Confidence")).toHaveClass("text-success");

    rerender(<ConfidenceIndicator score={60} />);
    expect(screen.getByText("Medium Confidence")).toHaveClass("text-warning");

    rerender(<ConfidenceIndicator score={20} />);
    expect(screen.getByText("Low Confidence")).toHaveClass("text-destructive");
  });

  it("uses warning token styling for volatility notices", () => {
    render(<ConfidenceIndicator score={85} volatility="high" />);
    expect(screen.getByText("Volatile")).toHaveClass("text-warning");
  });
});
