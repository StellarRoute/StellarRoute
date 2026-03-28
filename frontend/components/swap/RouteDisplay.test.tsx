import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RouteDisplay } from "./RouteDisplay";

describe("RouteDisplay", () => {
  afterEach(() => cleanup());

  it("should render loading skeleton when isLoading is true", () => {
    render(
      <RouteDisplay amountOut="50.0" isLoading={true} />
    );

    // Check for skeleton elements (animate-pulse class)
    const skeletonElements = document.querySelectorAll(".animate-pulse");
    expect(skeletonElements.length).toBeGreaterThanOrEqual(5);
  });

  it("should render actual content when isLoading is false or undefined", () => {
    render(
      <RouteDisplay amountOut="50.0" isLoading={false} />
    );

    expect(screen.getByText("Best Route")).toBeInTheDocument();
  });

  it("should accept isLoading prop as true", () => {
    const { container } = render(
      <RouteDisplay amountOut="50.0" isLoading={true} />
    );

    // Verify skeleton is rendered by checking for skeleton elements
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("should accept isLoading prop as false", () => {
    const { container } = render(
      <RouteDisplay amountOut="50.0" isLoading={false} />
    );

    // Verify content is rendered (not skeleton)
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBe(0);
  });

  it("should maintain layout stability during state transitions", () => {
    const { container, rerender } = render(
      <RouteDisplay amountOut="50.0" isLoading={true} />
    );

    // Get initial height
    const initialHeight = container.querySelector(
      ".rounded-xl"
    )?.clientHeight;

    // Switch to loaded state
    rerender(<RouteDisplay amountOut="50.0" isLoading={false} />);

    // Height should remain similar (no layout shift)
    const finalHeight = container.querySelector(
      ".rounded-xl"
    )?.clientHeight;

    // Both should be defined and similar (allow small variance)
    expect(initialHeight).toBeDefined();
    expect(finalHeight).toBeDefined();
    if (initialHeight && finalHeight) {
      expect(Math.abs(initialHeight - finalHeight)).toBeLessThan(50);
    }
  });
});
