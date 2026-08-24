import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import { useSlippageAcknowledgment } from "@/hooks/useSlippageAcknowledgment";
import { SlippageWarningBanner } from "./SlippageWarningBanner";

import { afterEach } from "vitest";
afterEach(() => cleanup());

// ── hook tests ──────────────────────────────────────────────────────────────

describe("useSlippageAcknowledgment", () => {
  it("returns safe tier for normal slippage (0.5%)", () => {
    const { result } = renderHook(() => useSlippageAcknowledgment(0.5));
    expect(result.current.tier).toBe("safe");
    expect(result.current.blocked).toBe(false);
    expect(result.current.message).toBeNull();
  });

  it("returns low tier below 0.1%", () => {
    const { result } = renderHook(() => useSlippageAcknowledgment(0.05));
    expect(result.current.tier).toBe("low");
    expect(result.current.blocked).toBe(false);
  });

  it("returns high tier above 1% and blocks submit until acknowledged", () => {
    const { result } = renderHook(() => useSlippageAcknowledgment(5));
    expect(result.current.tier).toBe("high");
    expect(result.current.blocked).toBe(true);
    act(() => result.current.acknowledge());
    expect(result.current.acknowledged).toBe(true);
    expect(result.current.blocked).toBe(false);
  });

  it("resets acknowledgment when resetKey changes", () => {
    const { result, rerender } = renderHook(
      ({ key }) => useSlippageAcknowledgment(5, key),
      { initialProps: { key: "XLM-USDC-100" } },
    );
    act(() => result.current.acknowledge());
    expect(result.current.acknowledged).toBe(true);

    rerender({ key: "XLM-USDC-200" });
    expect(result.current.acknowledged).toBe(false);
    expect(result.current.blocked).toBe(true);
  });

  it("resets acknowledgment when tier changes", () => {
    const { result, rerender } = renderHook(
      ({ slip }) => useSlippageAcknowledgment(slip),
      { initialProps: { slip: 5 } },
    );
    act(() => result.current.acknowledge());
    expect(result.current.acknowledged).toBe(true);

    // Drop to safe tier
    rerender({ slip: 0.5 });
    expect(result.current.tier).toBe("safe");
    // Raise back to high — acknowledgment should be reset
    rerender({ slip: 5 });
    expect(result.current.acknowledged).toBe(false);
  });
});

// ── component tests ─────────────────────────────────────────────────────────

describe("SlippageWarningBanner", () => {
  it("renders nothing for safe tier", () => {
    const state = { tier: "safe" as const, message: null, acknowledged: false, blocked: false, acknowledge: () => {} };
    const { container } = render(<SlippageWarningBanner state={state} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders low warning without acknowledge button", () => {
    const state = {
      tier: "low" as const,
      message: "Very low slippage may cause your swap to fail.",
      acknowledged: false,
      blocked: false,
      acknowledge: () => {},
    };
    render(<SlippageWarningBanner state={state} />);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("renders high warning with acknowledge button that blocks submit", () => {
    const acknowledge = vi.fn();
    const state = {
      tier: "high" as const,
      message: "High slippage — you may receive significantly less than expected.",
      acknowledged: false,
      blocked: true,
      acknowledge,
    };
    render(<SlippageWarningBanner state={state} />);
    const btn = screen.getByRole("button", { name: /acknowledge/i });
    fireEvent.click(btn);
    expect(acknowledge).toHaveBeenCalledOnce();
  });

  it("shows acknowledged confirmation text after acknowledgment", () => {
    const state = {
      tier: "high" as const,
      message: "High slippage",
      acknowledged: true,
      blocked: false,
      acknowledge: () => {},
    };
    render(<SlippageWarningBanner state={state} />);
    expect(screen.getByText(/risk acknowledged/i)).toBeInTheDocument();
    expect(screen.queryByRole("button")).toBeNull();
  });
});
