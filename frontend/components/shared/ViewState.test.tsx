import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LoadingState, ErrorState, EmptyState } from "./ViewState";

describe("LoadingState", () => {
  it("renders with default message", () => {
    render(<LoadingState />);
    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("renders with custom message", () => {
    render(<LoadingState message="Fetching quotes…" />);
    expect(screen.getByText("Fetching quotes…")).toBeInTheDocument();
  });

  it("sets role='status'", () => {
    render(<LoadingState />);
    expect(screen.getByRole("status")).toBeInTheDocument();
  });
});

describe("ErrorState", () => {
  it("renders error message", () => {
    render(<ErrorState message="Network error" />);
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("Network error")).toBeInTheDocument();
  });

  it("renders retry button when onRetry is provided", () => {
    const onRetry = vi.fn();
    render(<ErrorState message="Failed" onRetry={onRetry} />);
    const retryButton = screen.getByRole("button", { name: /retry/i });
    expect(retryButton).toBeInTheDocument();
  });

  it("calls onRetry when retry button is clicked", async () => {
    const onRetry = vi.fn();
    render(<ErrorState message="Failed" onRetry={onRetry} />);
    await userEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("does not render retry button when onRetry is omitted", () => {
    render(<ErrorState message="Failed" />);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("sets role='alert' for error variant", () => {
    render(<ErrorState message="Oops" />);
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});

describe("EmptyState", () => {
  it("renders with default message", () => {
    render(<EmptyState />);
    expect(screen.getByText("No data")).toBeInTheDocument();
    expect(screen.getByText("There is nothing to display yet.")).toBeInTheDocument();
  });

  it("renders with custom message and description", () => {
    render(
      <EmptyState
        message="No transactions"
        description="Your history will appear here."
      />,
    );
    expect(screen.getByText("No transactions")).toBeInTheDocument();
    expect(screen.getByText("Your history will appear here.")).toBeInTheDocument();
  });

  it("renders action when provided", () => {
    render(
      <EmptyState
        message="No items"
        action={<button>Create one</button>}
      />,
    );
    expect(screen.getByRole("button", { name: /create one/i })).toBeInTheDocument();
  });
});
