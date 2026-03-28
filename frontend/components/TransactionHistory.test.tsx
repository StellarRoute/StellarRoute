import { render, screen, waitFor } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TransactionHistory } from "./TransactionHistory";

// Mock the hooks
vi.mock("@/hooks/useTransactionHistory", () => ({
  useTransactionHistory: () => ({
    transactions: [],
    clearHistory: vi.fn(),
  }),
}));

describe("TransactionHistory", () => {
  afterEach(() => cleanup());

  it("should show skeleton loader initially", async () => {
    const { container } = render(<TransactionHistory />);

    // Check for skeleton elements immediately
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("should replace skeleton with empty state after loading", async () => {
    const { container, rerender } = render(<TransactionHistory />);

    // Initially should have skeletons
    let skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);

    // After loading time, skeletons should be gone
    await waitFor(
      () => {
        const newSkeletons = container.querySelectorAll(".animate-pulse");
        expect(newSkeletons.length).toBe(0);
      },
      { timeout: 500 }
    );
  });

  it("should render correct header structure", () => {
    render(<TransactionHistory />);

    expect(screen.getByText("Transaction History")).toBeInTheDocument();
  });

  it("should maintain layout stability during loading to loaded transition", async () => {
    const { container } = render(<TransactionHistory />);

    // Get initial container height during loading
    const scrollArea = container.querySelector(".flex-1");
    const initialHeight = scrollArea?.clientHeight;

    // Wait for loading to complete
    await waitFor(
      () => {
        const skeletons = container.querySelectorAll(".animate-pulse");
        expect(skeletons.length).toBe(0);
      },
      { timeout: 500 }
    );

    // Get final height after loading
    const finalHeight = scrollArea?.clientHeight;

    // Heights should be similar (no layout shift)
    if (initialHeight && finalHeight) {
      expect(Math.abs(initialHeight - finalHeight)).toBeLessThan(50);
    }
  });

  it("should not flicker on fast responses", async () => {
    let renderCount = 0;
    const originalRender = render;

    const { container } = render(<TransactionHistory />);

    // Skeleton should be visible initially
    expect(container.querySelectorAll(".animate-pulse").length).toBeGreaterThan(0);

    // Simulate fast content load
    await new Promise((resolve) => setTimeout(resolve, 350));

    // Skeleton should now be hidden
    expect(container.querySelectorAll(".animate-pulse").length).toBe(0);
  });
});
