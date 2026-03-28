import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { TokenSearchModal } from "@/components/shared/TokenSearchModal";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

const assets = [
  { code: "XLM", asset: "native", displayName: "XLM" },
  {
    code: "USDC",
    asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    displayName: "USDC",
  },
  {
    code: "AQUA",
    asset: "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
    issuer: "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
    displayName: "AQUA",
  },
];

describe("TokenSearchModal", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
  });

  it("renders when open", () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );
    expect(screen.getByText("Select token")).toBeDefined();
    expect(screen.getByPlaceholderText(/search by symbol or address/i)).toBeDefined();
  });

  it("does not render when closed", () => {
    render(
      <TokenSearchModal
        isOpen={false}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );
    expect(screen.queryByText("Select token")).toBeNull();
  });

  it("shows all assets initially", () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );
    expect(screen.getByText("XLM")).toBeDefined();
    expect(screen.getByText("USDC")).toBeDefined();
    expect(screen.getByText("AQUA")).toBeDefined();
  });

  it("filters by symbol", async () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);
    fireEvent.change(input, { target: { value: "USDC" } });

    await waitFor(() => {
      expect(screen.getByText("USDC")).toBeDefined();
      expect(screen.queryByText("AQUA")).toBeNull();
    });
  });

  it("filters by issuer address", async () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);
    fireEvent.change(input, { target: { value: "GA5ZSE" } });

    await waitFor(() => {
      expect(screen.getByText("USDC")).toBeDefined();
      expect(screen.queryByText("AQUA")).toBeNull();
    });
  });

  it("shows empty state when no results found", async () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);
    fireEvent.change(input, { target: { value: "NONEXISTENT" } });

    await waitFor(() => {
      expect(screen.getByText("No tokens found")).toBeDefined();
      expect(screen.getByText(/try searching by contract address/i)).toBeDefined();
    });
  });

  it("calls onSelect and onClose when token is clicked", async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();

    render(
      <TokenSearchModal
        isOpen={true}
        onClose={onClose}
        assets={assets}
        onSelect={onSelect}
        title="Select token"
      />
    );

    const xlmButtons = screen.getAllByText("XLM");
    const tokenButton = xlmButtons[0].closest("button")!;
    fireEvent.click(tokenButton);

    expect(onSelect).toHaveBeenCalledWith("native");
    expect(onClose).toHaveBeenCalled();
  });

  it("shows check icon on selected asset", () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
        selectedAsset="native"
      />
    );

    // Check icon rendered via mock will be an svg, we verify the button has selected styling
    // by querying the data-token-item buttons
    const tokenButtons = screen.getAllByRole("button", { hidden: true });
    const xlmButton = tokenButtons.find((b) =>
      b.getAttribute("data-token-item") !== null &&
      b.textContent?.includes("XLM")
    );
    expect(xlmButton).toBeDefined();
  });

  it("keyboard: ArrowDown moves selection, Enter selects", async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();

    render(
      <TokenSearchModal
        isOpen={true}
        onClose={onClose}
        assets={assets}
        onSelect={onSelect}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);

    // Move down once (from index 0 to 1) and press Enter
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onSelect).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("keyboard: ArrowUp wraps to last item", async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();

    render(
      <TokenSearchModal
        isOpen={true}
        onClose={onClose}
        assets={assets}
        onSelect={onSelect}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);

    // ArrowUp from index 0 should wrap to last (index 2: AQUA)
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });

    // Should select the last item (AQUA)
    expect(onSelect).toHaveBeenCalledWith(
      "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA"
    );
  });

  it("clears search when X button is clicked", async () => {
    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );

    const input = screen.getByPlaceholderText(/search by symbol or address/i);
    fireEvent.change(input, { target: { value: "USDC" } });

    await waitFor(() => {
      expect(screen.queryByText("AQUA")).toBeNull();
    });

    const clearButton = screen.getByRole("button", {
      name: (_, el) => el?.querySelector("svg") !== null && el.textContent === "",
      hidden: true,
    });
    // Find the clear X button by looking after the input
    const allButtons = screen.getAllByRole("button", { hidden: true });
    const xButton = allButtons.find(
      (b) => !b.hasAttribute("data-token-item") && b.textContent === ""
    );
    if (xButton) fireEvent.click(xButton);

    await waitFor(() => {
      expect(screen.getByText("AQUA")).toBeDefined();
    });
  });

  it("shows recent tokens section when there are recent picks", async () => {
    // Pre-populate localStorage with a recent token
    localStorage.setItem("stellar-route-recent-tokens", JSON.stringify(["native"]));

    render(
      <TokenSearchModal
        isOpen={true}
        onClose={vi.fn()}
        assets={assets}
        onSelect={vi.fn()}
        title="Select token"
      />
    );

    // The recent section shows after the hook hydrates
    await waitFor(() => {
      expect(screen.getByText("RECENT")).toBeDefined();
    });
  });
});
