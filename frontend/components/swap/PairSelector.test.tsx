import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SettingsProvider } from "@/components/providers/settings-provider";
import { PairSelector } from "./PairSelector";

vi.mock("next-themes", () => ({
  useTheme: () => ({
    theme: "light",
    setTheme: vi.fn(),
  }),
}));

describe("PairSelector", () => {
  afterEach(() => cleanup());

  it("keeps amount inputs keyboard-focusable with visible theme rings", () => {
    render(
      <SettingsProvider>
        <PairSelector payAmount="10" onPayAmountChange={vi.fn()} receiveAmount="9.8" />
      </SettingsProvider>,
    );

    expect(screen.getByLabelText("Pay amount")).toHaveClass(
      "focus-visible:ring-ring/50",
      "focus-visible:ring-[3px]",
    );
    expect(screen.getByLabelText("Receive amount")).toHaveClass(
      "focus-visible:ring-ring/50",
      "focus-visible:ring-[3px]",
    );
  });

  it("uses theme token styling for the receive token pill", () => {
    const { container } = render(
      <SettingsProvider>
        <PairSelector payAmount="10" onPayAmountChange={vi.fn()} receiveAmount="9.8" />
      </SettingsProvider>,
    );

    const receiveTokenAvatar = container.querySelector(".bg-primary\\/15");
    expect(receiveTokenAvatar).toHaveClass("text-primary");
  });
});
