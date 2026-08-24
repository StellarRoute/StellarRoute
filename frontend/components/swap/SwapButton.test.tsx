import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SwapButton } from "./SwapButton";

describe("SwapButton", () => {
  afterEach(() => cleanup());

  it("gates submission while high slippage is unacknowledged", () => {
    const onSwap = vi.fn();
    render(<SwapButton state="slippage_ack_required" onSwap={onSwap} />);

    const button = screen.getByRole("button", { name: /acknowledge slippage/i });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-disabled", "true");

    fireEvent.click(button);
    expect(onSwap).not.toHaveBeenCalled();
  });

  it("surfaces the disabled reason via a tooltip for blocked states", () => {
    render(<SwapButton state="insufficient_balance" onSwap={() => {}} />);

    const button = screen.getByRole("button", { name: /insufficient/i });
    expect(button).toHaveAttribute("aria-disabled", "true");
  });

  it("allows submission when ready", () => {
    const onSwap = vi.fn();
    render(<SwapButton state="ready" onSwap={onSwap} />);

    const button = screen.getByRole("button", { name: /review swap/i });
    expect(button).toHaveAttribute("aria-disabled", "false");

    fireEvent.click(button);
    expect(onSwap).toHaveBeenCalledTimes(1);
  });
});
