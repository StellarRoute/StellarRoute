import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { RouteDisplay } from "./RouteDisplay";

describe("RouteDisplay", () => {
  afterEach(() => cleanup());

  it("should render loading skeleton when isLoading is true", () => {
    render(<RouteDisplay amountOut="50.0" isLoading={true} />);

    const skeletonElements = document.querySelectorAll(".animate-pulse");
    expect(skeletonElements.length).toBeGreaterThanOrEqual(5);
  });

  it("should render actual content when isLoading is false or undefined", () => {
    render(<RouteDisplay amountOut="50.0" isLoading={false} />);

    expect(screen.getByText("Best Route")).toBeInTheDocument();
  });

  it("should accept isLoading prop as true", () => {
    const { container } = render(<RouteDisplay amountOut="50.0" isLoading={true} />);

    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("should accept isLoading prop as false", () => {
    const { container } = render(<RouteDisplay amountOut="50.0" isLoading={false} />);

    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBe(0);
  });

  it("should maintain layout stability during state transitions", () => {
    const { container, rerender } = render(<RouteDisplay amountOut="50.0" isLoading={true} />);

    const initialHeight = container.querySelector(".rounded-xl")?.clientHeight;

    rerender(<RouteDisplay amountOut="50.0" isLoading={false} />);

    const finalHeight = container.querySelector(".rounded-xl")?.clientHeight;

    expect(initialHeight).toBeDefined();
    expect(finalHeight).toBeDefined();
    if (initialHeight && finalHeight) {
      expect(Math.abs(initialHeight - finalHeight)).toBeLessThan(50);
    }
  });

  it("virtualizes long alternative route lists and updates the window on scroll", async () => {
    const routes = Array.from({ length: 20 }, (_, index) => ({
      id: `route-${index}`,
      venue: `Pool ${index}`,
      expectedAmount: `≈ ${(50 - index * 0.1).toFixed(4)}`,
    }));

    render(
      <RouteDisplay
        amountOut="50.0"
        alternativeRoutes={routes}
      />,
    );

    const initialButtons = screen.getAllByTestId(/alternative-route-route-/);
    expect(initialButtons.length).toBeLessThan(routes.length);
    expect(screen.getByTestId("alternative-route-route-0")).toBeInTheDocument();

    const scrollContainer = screen.getByTestId("alternative-routes-scroll");
    scrollContainer.scrollTop = 360;
    fireEvent.scroll(scrollContainer);

    await waitFor(() => {
      expect(screen.getByTestId("alternative-route-route-8")).toBeInTheDocument();
    });

    expect(screen.queryByTestId("alternative-route-route-0")).not.toBeInTheDocument();
  });
});
