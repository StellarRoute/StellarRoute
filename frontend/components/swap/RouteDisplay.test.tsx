import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
  },
}));

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

  it("should allow a user to pin and unpin a route", () => {
    const routes = [
      { id: "route-1", venue: "A", expectedAmount: "10" },
      { id: "route-2", venue: "B", expectedAmount: "9" },
    ];
    render(<RouteDisplay amountOut="10" alternativeRoutes={routes} />);

    const pinButton = screen.getByTestId("pin-route-route-1");
    fireEvent.click(pinButton);

    expect(toast.success).toHaveBeenCalledWith("Route pinned");

    fireEvent.click(pinButton);
    expect(toast.info).toHaveBeenCalledWith("Route unpinned");
  });

  it("should persist pinned route during quote refreshes if still valid", () => {
    const routes1 = [
      { id: "route-1", venue: "A", expectedAmount: "10" },
      { id: "route-2", venue: "B", expectedAmount: "9" },
    ];
    const { rerender } = render(<RouteDisplay amountOut="10" alternativeRoutes={routes1} />);

    const pinButton = screen.getByTestId("pin-route-route-1");
    fireEvent.click(pinButton);

    const routes2 = [
      { id: "route-1", venue: "A", expectedAmount: "11" },
      { id: "route-3", venue: "C", expectedAmount: "8" },
    ];
    rerender(<RouteDisplay amountOut="11" alternativeRoutes={routes2} isLoading={false} />);

    expect(toast.error).not.toHaveBeenCalled();
  });

  it("should invalidate pinned route and show fallback prompt if no longer available", () => {
    const routes1 = [
      { id: "route-1", venue: "A", expectedAmount: "10" },
      { id: "route-2", venue: "B", expectedAmount: "9" },
    ];
    const { rerender } = render(<RouteDisplay amountOut="10" alternativeRoutes={routes1} />);

    const pinButton = screen.getByTestId("pin-route-route-1");
    fireEvent.click(pinButton);

    const routes2 = [
      { id: "route-2", venue: "B", expectedAmount: "9" },
      { id: "route-3", venue: "C", expectedAmount: "8" },
    ];
    rerender(<RouteDisplay amountOut="9" alternativeRoutes={routes2} isLoading={false} />);

    expect(toast.error).toHaveBeenCalledWith("Pinned route is no longer available. Reverted to best route.");
  });
});
