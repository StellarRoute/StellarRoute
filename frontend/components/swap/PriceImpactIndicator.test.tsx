import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PriceImpactIndicator } from "./PriceImpactIndicator";

describe("PriceImpactIndicator", () => {
  it("renders safe impact (< 1%) in emerald color", () => {
    const { container } = render(<PriceImpactIndicator impact={0.5} />);
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveClass("text-emerald-500");
    expect(screen.getByText("0.50%")).toBeInTheDocument();
  });

  it("renders moderate impact (1-3%) in yellow color", () => {
    const { container } = render(<PriceImpactIndicator impact={2.5} />);
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveClass("text-yellow-500");
    expect(screen.getByText("2.50%")).toBeInTheDocument();
  });

  it("renders high impact (3-5%) in orange color", () => {
    const { container } = render(<PriceImpactIndicator impact={4.5} />);
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveClass("text-orange-500");
    expect(screen.getByText("4.50%")).toBeInTheDocument();
  });

  it("renders very high impact (> 5%) in destructive color", () => {
    const { container } = render(<PriceImpactIndicator impact={6.0} />);
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveClass("text-destructive");
    expect(screen.getByText("6.00%")).toBeInTheDocument();
  });

  it("renders small impact fallback for 0%", () => {
    render(<PriceImpactIndicator impact={0} />);
    expect(screen.getByText("< 0.01%")).toBeInTheDocument();
  });
});
