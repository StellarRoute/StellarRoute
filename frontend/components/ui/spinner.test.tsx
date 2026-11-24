import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Spinner } from "./spinner";

function setReducedMotion(value: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: () => ({
      matches: value,
      media: "(prefers-reduced-motion: reduce)",
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(() => false),
    }),
  });
}

describe("Spinner", () => {
  afterEach(cleanup);

  it("animates when the user has no reduced-motion preference", () => {
    setReducedMotion(false);
    render(<Spinner />);

    expect(screen.getByRole("status")).toHaveClass("animate-spin");
  });

  it("falls back to a static icon when prefers-reduced-motion is set", () => {
    setReducedMotion(true);
    render(<Spinner />);

    expect(screen.getByRole("status")).not.toHaveClass("animate-spin");
  });

  it("exposes an accessible label for screen readers", () => {
    setReducedMotion(false);
    render(<Spinner label="Loading quote" />);

    expect(screen.getByRole("status")).toHaveAttribute("aria-label", "Loading quote");
  });
});
