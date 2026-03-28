import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { TokenPairSelector } from "./TokenPairSelector";
import type { TradingPair } from "@/types";

// Mock toast
vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockPairs: TradingPair[] = [
  {
    base: "XLM",
    counter: "USDC",
    base_asset: "native",
    counter_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    offer_count: 100,
    last_updated: "2024-01-01T00:00:00Z",
  },
  {
    base: "USDC",
    counter: "XLM",
    base_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    counter_asset: "native",
    offer_count: 100,
    last_updated: "2024-01-01T00:00:00Z",
  },
  {
    base: "XLM",
    counter: "AQUA",
    base_asset: "native",
    counter_asset: "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
    offer_count: 50,
    last_updated: "2024-01-01T00:00:00Z",
  },
  {
    base: "USDC",
    counter: "AQUA",
    base_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    counter_asset: "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
    offer_count: 25,
    last_updated: "2024-01-01T00:00:00Z",
  },
];

describe("TokenPairSelector", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders base and quote asset buttons", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        selectedQuote="USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        onPairChange={onPairChange}
      />
    );

    expect(screen.getByText("You sell")).toBeDefined();
    expect(screen.getByText("You buy")).toBeDefined();
    expect(screen.getByText("XLM")).toBeDefined();
    expect(screen.getByText("USDC")).toBeDefined();
  });

  it("opens base asset dialog when clicking sell button", async () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        onPairChange={onPairChange}
      />
    );

    const buttons = screen.getAllByText("You sell");
    fireEvent.click(buttons[0].closest("button")!);

    await waitFor(() => {
      expect(screen.getByText("Select asset to sell")).toBeDefined();
    });
  });

  it("opens quote asset dialog when clicking buy button", async () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        onPairChange={onPairChange}
      />
    );

    const buttons = screen.getAllByText("You buy");
    fireEvent.click(buttons[0].closest("button")!);

    await waitFor(() => {
      expect(screen.getByText("Select asset to buy")).toBeDefined();
    });
  });

  it("filters assets based on search input", async () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        onPairChange={onPairChange}
      />
    );

    const buttons = screen.getAllByText("You sell");
    fireEvent.click(buttons[0].closest("button")!);

    await waitFor(() => {
      expect(screen.getByText("Select asset to sell")).toBeDefined();
    });

    const searchInput = screen.getByPlaceholderText(/search by symbol or address/i);
    fireEvent.change(searchInput, { target: { value: "USDC" } });

    await waitFor(() => {
      expect(screen.getByText("USDC")).toBeDefined();
      expect(screen.queryByText("AQUA")).toBeNull();
    });
  });

  it("calls onPairChange when selecting an asset", async () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        onPairChange={onPairChange}
      />
    );

    const buttons = screen.getAllByText("You sell");
    fireEvent.click(buttons[0].closest("button")!);

    await waitFor(() => {
      expect(screen.getByText("Select asset to sell")).toBeDefined();
    });

    const xlmOptions = screen.getAllByText("XLM");
    // Click the one inside the dialog
    fireEvent.click(xlmOptions[xlmOptions.length - 1].closest("button")!);

    await waitFor(() => {
      expect(onPairChange).toHaveBeenCalledWith("native", "");
    });
  });

  it("shows invalid pair message when pair is not available", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        selectedQuote="INVALID:ISSUER"
        onPairChange={onPairChange}
      />
    );

    expect(screen.getByText("Invalid pair selection")).toBeDefined();
  });

  it("swaps base and quote when swap button is clicked", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        selectedQuote="USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        onPairChange={onPairChange}
      />
    );

    const swapButton = screen.getByTitle("Swap base and quote assets");
    fireEvent.click(swapButton);

    expect(onPairChange).toHaveBeenCalledWith(
      "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "native"
    );
  });

  it("disables swap button when pair cannot be reversed", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={[mockPairs[0]]} // Only XLM/USDC, not USDC/XLM
        selectedBase="native"
        selectedQuote="USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        onPairChange={onPairChange}
      />
    );

    const swapButton = screen.getByTitle("Swap base and quote assets");
    expect(swapButton).toHaveProperty("disabled", true);
  });

  it("truncates long issuer addresses", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        selectedQuote="USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        onPairChange={onPairChange}
      />
    );

    // Issuer should be truncated in the display
    expect(screen.getByText("GA5ZSE...KZVN")).toBeDefined();
  });

  it("shows error message when provided", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        onPairChange={onPairChange}
        error="API connection failed"
      />
    );

    expect(screen.getByText("API connection failed")).toBeDefined();
  });

  it("disables buttons when loading", () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        onPairChange={onPairChange}
        loading={true}
      />
    );

    const sellButtons = screen.getAllByText("You sell");
    const buyButtons = screen.getAllByText("You buy");

    expect(sellButtons[0].closest("button")).toHaveProperty("disabled", true);
    expect(buyButtons[0].closest("button")).toHaveProperty("disabled", true);
  });

  it("filters quote assets based on selected base", async () => {
    const onPairChange = vi.fn();
    render(
      <TokenPairSelector
        pairs={mockPairs}
        selectedBase="native"
        onPairChange={onPairChange}
      />
    );

    const buyButtons = screen.getAllByText("You buy");
    fireEvent.click(buyButtons[0].closest("button")!);

    await waitFor(() => {
      expect(screen.getByText("Select asset to buy")).toBeDefined();
    });

    // Should show USDC and AQUA (both pair with XLM)
    const usdcElements = screen.getAllByText("USDC");
    const aquaElements = screen.getAllByText("AQUA");
    expect(usdcElements.length).toBeGreaterThan(0);
    expect(aquaElements.length).toBeGreaterThan(0);
  });
});
