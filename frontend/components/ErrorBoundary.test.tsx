import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ErrorBoundary from "./ErrorBoundary";
import React from "react";

const ThrowError = ({ message }: { message: string }) => {
  throw new Error(message);
};

describe("ErrorBoundary", () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  it("renders children when there is no error", () => {
    render(
      <ErrorBoundary>
        <div>Test Content</div>
      </ErrorBoundary>
    );

    expect(screen.getByText("Test Content")).toBeDefined();
  });

  it("renders error UI when a child throws", () => {
    render(
      <ErrorBoundary>
        <ThrowError message="Test Error" />
      </ErrorBoundary>
    );

    expect(screen.getByText("Oops! Something went wrong")).toBeDefined();
    expect(screen.getByText("Test Error")).toBeDefined();
  });

  it("reloads the page when refresh button is clicked", () => {
    const originalLocation = window.location;
    // @ts-ignore
    delete window.location;
    window.location = { ...originalLocation, reload: vi.fn() };

    render(
      <ErrorBoundary>
        <ThrowError message="Test Error" />
      </ErrorBoundary>
    );

    fireEvent.click(screen.getAllByRole("button", { name: /refresh/i })[0]);


    expect(window.location.reload).toHaveBeenCalled();

    window.location = originalLocation;
  });
});
