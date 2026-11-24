import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsProvider } from "@/components/providers/settings-provider";
import { PairSelector } from "./PairSelector";

vi.mock("next-themes", () => ({
  useTheme: () => ({
    theme: "light",
    setTheme: vi.fn(),
  }),
}));

const RECENT_KEY = "stellar-route-recent-pairs";

function renderPairSelector(props?: Partial<React.ComponentProps<typeof PairSelector>>) {
  return render(
    <SettingsProvider>
      <PairSelector
        payAmount="10"
        onPayAmountChange={vi.fn()}
        receiveAmount="9.8"
        {...props}
      />
    </SettingsProvider>,
  );
}

describe("PairSelector", () => {
  afterEach(() => cleanup());
  beforeEach(() => localStorage.clear());

  it("keeps amount inputs keyboard-focusable with visible theme rings", () => {
    renderPairSelector();

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
    const { container } = renderPairSelector();

    const receiveTokenAvatar = container.querySelector(".bg-primary\\/15");
    expect(receiveTokenAvatar).toHaveClass("text-primary");
  });

  // ── Recent pairs LRU (issue #1733) ────────────────────────────────────────

  it("does not render the recent pairs section when history is empty", () => {
    renderPairSelector();
    expect(screen.queryByRole("navigation", { name: /recent pairs/i })).not.toBeInTheDocument();
    expect(screen.queryByTestId("recent-pairs-list")).not.toBeInTheDocument();
  });

  it("renders recent pairs from localStorage on mount", () => {
    localStorage.setItem(
      RECENT_KEY,
      JSON.stringify([
        { base: "XLM", quote: "USDC", baseAsset: "native", quoteAsset: "usdc-asset" },
      ]),
    );

    renderPairSelector();

    const nav = screen.getByRole("navigation", { name: /recent pairs/i });
    expect(nav).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /select recent pair XLM\/USDC/i })).toBeInTheDocument();
  });

  it("renders all stored recent pairs as buttons", () => {
    localStorage.setItem(
      RECENT_KEY,
      JSON.stringify([
        { base: "XLM", quote: "USDC", baseAsset: "native", quoteAsset: "usdc-asset" },
        { base: "XLM", quote: "EURC", baseAsset: "native", quoteAsset: "eurc-asset" },
      ]),
    );

    renderPairSelector();

    expect(screen.getByRole("button", { name: /XLM\/USDC/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /XLM\/EURC/i })).toBeInTheDocument();
  });

  it("calls onRecentPairSelect with the pair when a recent pair button is clicked", async () => {
    localStorage.setItem(
      RECENT_KEY,
      JSON.stringify([
        { base: "XLM", quote: "USDC", baseAsset: "native", quoteAsset: "usdc-asset" },
      ]),
    );

    const onSelect = vi.fn();
    renderPairSelector({ onRecentPairSelect: onSelect });

    const button = screen.getByRole("button", { name: /XLM\/USDC/i });
    button.click();

    expect(onSelect).toHaveBeenCalledOnce();
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({ base: "XLM", quote: "USDC" }),
    );
  });
});
