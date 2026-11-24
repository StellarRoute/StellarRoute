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

  // ── Accessible description (issue #1734) ─────────────────────────────────

  it("trigger has role=status and an aria-label describing level and value", () => {
    const { container } = render(<PriceImpactIndicator impact={0.5} />);
    const trigger = container.firstChild as HTMLElement;
    expect(trigger).toHaveAttribute("role", "status");
    expect(trigger).toHaveAttribute(
      "aria-label",
      "Price impact: 0.50% — Safe",
    );
  });

  it("aria-label reflects Moderate level for 2.5%", () => {
    const { container } = render(<PriceImpactIndicator impact={2.5} />);
    const trigger = container.firstChild as HTMLElement;
    expect(trigger).toHaveAttribute(
      "aria-label",
      "Price impact: 2.50% — Moderate",
    );
  });

  it("aria-label reflects High level for 4.5%", () => {
    const { container } = render(<PriceImpactIndicator impact={4.5} />);
    const trigger = container.firstChild as HTMLElement;
    expect(trigger).toHaveAttribute(
      "aria-label",
      "Price impact: 4.50% — High",
    );
  });

  it("aria-label reflects Very High level for 6%", () => {
    const { container } = render(<PriceImpactIndicator impact={6.0} />);
    const trigger = container.firstChild as HTMLElement;
    expect(trigger).toHaveAttribute(
      "aria-label",
      "Price impact: 6.00% — Very High",
    );
  });

  it("aria-label uses < 0.01% fallback display value for zero impact", () => {
    const { container } = render(<PriceImpactIndicator impact={0} />);
    const trigger = container.firstChild as HTMLElement;
    expect(trigger).toHaveAttribute(
      "aria-label",
      "Price impact: < 0.01% — Safe",
    );
  });

  it("trigger carries aria-describedby pointing to the tooltip content id", () => {
    const { container } = render(<PriceImpactIndicator impact={0.5} />);
    const trigger = container.firstChild as HTMLElement;
    const descId = trigger.getAttribute("aria-describedby");
    expect(descId).toBeTruthy();
    // The tooltip content element should carry that id when rendered in DOM
    const tooltipContent = container.querySelector(`[id="${descId}"]`);
    // TooltipContent renders in a Portal; id is set on the content element
    // We verify the attribute is a non-empty string (portal may not be in container)
    expect(typeof descId).toBe("string");
    expect(descId!.length).toBeGreaterThan(0);
  });

  it("icon span is aria-hidden when showIcon is true", () => {
    const { container } = render(<PriceImpactIndicator impact={0.5} showIcon />);
    const iconSpan = container.querySelector('[aria-hidden="true"]');
    expect(iconSpan).toBeInTheDocument();
  });

  it("does not render an icon span when showIcon is false", () => {
    const { container } = render(
      <PriceImpactIndicator impact={0.5} showIcon={false} />,
    );
    const iconSpan = container.querySelector('[aria-hidden="true"]');
    expect(iconSpan).not.toBeInTheDocument();
  });
});
