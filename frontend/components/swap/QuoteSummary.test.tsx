import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { QuoteSummary } from "./QuoteSummary";

describe("QuoteSummary", () => {
  afterEach(() => cleanup());

  it("should render loading skeleton when isLoading is true", () => {
    render(
      <QuoteSummary
        rate="1 XLM ≈ 0.98 USDC"
        fee="0.01 XLM"
        priceImpact="< 0.1%"
        isLoading={true}
      />
    );

    // Check for skeleton elements (animate-pulse class)
    const skeletonElements = document.querySelectorAll(".animate-pulse");
    expect(skeletonElements.length).toBeGreaterThanOrEqual(3);
  });

  it("should render actual content when isLoading is false", () => {
    render(
      <QuoteSummary
        rate="1 XLM ≈ 0.98 USDC"
        fee="0.01 XLM"
        priceImpact="< 0.1%"
        isLoading={false}
      />
    );

    expect(screen.getByText("Rate")).toBeInTheDocument();
    expect(screen.getByText("1 XLM ≈ 0.98 USDC")).toBeInTheDocument();
    expect(screen.getByText("Network Fee")).toBeInTheDocument();
    expect(screen.getByText("0.01 XLM")).toBeInTheDocument();
    expect(screen.getByText("Price Impact")).toBeInTheDocument();
    expect(screen.getByText("< 0.1%")).toBeInTheDocument();
  });

  it("should maintain layout stability with skeleton", () => {
    const { container: skeletonContainer } = render(
      <QuoteSummary
        rate="1 XLM ≈ 0.98 USDC"
        fee="0.01 XLM"
        priceImpact="< 0.1%"
        isLoading={true}
      />
    );

    const { container: contentContainer } = render(
      <QuoteSummary
        rate="1 XLM ≈ 0.98 USDC"
        fee="0.01 XLM"
        priceImpact="< 0.1%"
        isLoading={false}
      />
    );

    // Both should have the same root structure class
    const skeletonRoot = skeletonContainer.querySelector(
      ".rounded-xl.border.border-border\\/50.p-4.space-y-3"
    );
    const contentRoot = contentContainer.querySelector(
      ".rounded-xl.border.border-border\\/50.p-4.space-y-3"
    );

    expect(skeletonRoot).toBeInTheDocument();
    expect(contentRoot).toBeInTheDocument();
  });

  it("should not show loading skeleton by default", () => {
    render(
      <QuoteSummary
        rate="1 XLM ≈ 0.98 USDC"
        fee="0.01 XLM"
        priceImpact="< 0.1%"
      />
    );

    expect(screen.getByText("1 XLM ≈ 0.98 USDC")).toBeInTheDocument();
  });
});
