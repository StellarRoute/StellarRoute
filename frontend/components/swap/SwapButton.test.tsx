import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { SwapButton } from "./SwapButton";

describe("SwapButton", () => {
  afterEach(() => cleanup());

  it("gates submission while high slippage is unacknowledged", () => {
    render(
      <SwapButton
        state="slippage_ack_required"
        onSwap={() => {}}
      />,
    );

    expect(
      screen.getByRole("button", { name: /acknowledge slippage/i }),
    ).toBeDisabled();
  });
});
