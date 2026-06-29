import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { SwapStateView } from "./SwapStateView";

afterEach(() => cleanup());

describe("SwapStateView", () => {
  const contexts = ["quote", "routes", "history", "wallet"] as const;
  const variants = ["loading", "empty", "error"] as const;

  // Smoke test: all context × variant combos render without throwing
  for (const context of contexts) {
    for (const variant of variants) {
      it(`renders ${context}/${variant} without error`, () => {
        const { container } = render(<SwapStateView context={context} variant={variant} />);
        expect(container.firstChild).not.toBeNull();
      });
    }
  }

  it("shows retry button on error when onRetry provided", () => {
    const onRetry = vi.fn();
    render(<SwapStateView context="quote" variant="error" onRetry={onRetry} />);
    const btn = screen.getByRole("button", { name: /try again/i });
    fireEvent.click(btn);
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("does not show retry button when onRetry is absent", () => {
    render(<SwapStateView context="quote" variant="error" />);
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("uses custom errorMessage when provided", () => {
    render(<SwapStateView context="routes" variant="error" errorMessage="Custom error text" />);
    expect(screen.getByText("Custom error text")).toBeInTheDocument();
  });

  it("uses role=alert for error variant", () => {
    render(<SwapStateView context="history" variant="error" />);
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("uses role=status for loading variant", () => {
    render(<SwapStateView context="wallet" variant="loading" />);
    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("shows correct copy for quote/empty", () => {
    render(<SwapStateView context="quote" variant="empty" />);
    expect(screen.getByText("No quote yet")).toBeInTheDocument();
    expect(screen.getByText("Enter an amount to see a price quote.")).toBeInTheDocument();
  });

  it("shows correct copy for history/empty", () => {
    render(<SwapStateView context="history" variant="empty" />);
    expect(screen.getByText("No transactions yet")).toBeInTheDocument();
  });
});
