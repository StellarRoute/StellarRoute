/**
 * Tests for the developer debug overlay (issue #517).
 *
 * NOTE: The production gate (`process.env.NODE_ENV === 'production'`)
 * is tested separately by setting the env variable before importing the
 * module. Here we test the default test/development behaviour.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DebugOverlay } from "./DebugOverlay";

// useSearchParams must be mocked because we're not running inside Next.js
vi.mock("next/navigation", () => ({
  useSearchParams: () => new URLSearchParams(),
}));

describe("DebugOverlay", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("is hidden by default (no query param, no keyboard shortcut)", () => {
    render(<DebugOverlay />);
    expect(screen.queryByTestId("debug-overlay")).not.toBeInTheDocument();
  });

  it("becomes visible when Ctrl+Shift+D is pressed", async () => {
    render(<DebugOverlay />);
    expect(screen.queryByTestId("debug-overlay")).not.toBeInTheDocument();

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();
  });

  it("becomes visible when Cmd+Shift+D is pressed (macOS)", async () => {
    render(<DebugOverlay />);

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          metaKey: true,
          bubbles: true,
        })
      );
    });

    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();
  });

  it("toggles off when shortcut is pressed a second time", async () => {
    render(<DebugOverlay />);

    const fireD = () =>
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );

    await act(async () => { fireD(); });
    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();

    await act(async () => { fireD(); });
    expect(screen.queryByTestId("debug-overlay")).not.toBeInTheDocument();
  });

  it("closes when the ✕ button is clicked", async () => {
    const user = userEvent.setup();
    render(<DebugOverlay />);

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /close debug overlay/i }));
    expect(screen.queryByTestId("debug-overlay")).not.toBeInTheDocument();
  });

  it("displays quote ID and snapshot version from info prop", async () => {
    render(
      <DebugOverlay
        info={{ quoteId: "qid-abc123", snapshotVersion: "v42" }}
      />
    );

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    expect(screen.getByText("qid-abc123")).toBeInTheDocument();
    expect(screen.getByText("v42")).toBeInTheDocument();
  });

  it("displays timing entries", async () => {
    render(
      <DebugOverlay
        info={{ timings: { quoteLatency: 123.4, renderTime: 5.6 } }}
      />
    );

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    expect(screen.getByText("123.4 ms")).toBeInTheDocument();
    expect(screen.getByText("5.6 ms")).toBeInTheDocument();
  });

  it("masks Stellar wallet addresses (G…)", async () => {
    const stellarAddress = "GABC1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMN";
    render(
      <DebugOverlay
        info={{ quoteId: stellarAddress }}
      />
    );

    await act(async () => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "D",
          shiftKey: true,
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    // Full address should NOT appear
    expect(screen.queryByText(stellarAddress)).not.toBeInTheDocument();
    // Masked form should appear
    expect(screen.getByText("GABC…KLMN")).toBeInTheDocument();
  });
});
