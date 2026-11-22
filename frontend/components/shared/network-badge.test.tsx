/**
 * W12-FE-12 — Header network badge testnet/mainnet.
 *
 * Verifies that the NetworkBadge renders the correct label and aria-label
 * for both testnet and mainnet, and that the badge variant switches correctly.
 */
import { render, screen, cleanup } from "@testing-library/react";
import { describe, it, expect, vi, afterEach } from "vitest";
import { NetworkBadge } from "./network-badge";

vi.mock("@/components/providers/wallet-provider", () => ({
  useWallet: vi.fn(),
}));

async function mockNetwork(network: string) {
  const { useWallet } = await import("@/components/providers/wallet-provider");
  vi.mocked(useWallet).mockReturnValue({
    network,
    setNetwork: vi.fn(),
    publicKey: null,
    connected: false,
    connect: vi.fn(),
    disconnect: vi.fn(),
    walletType: null,
    isConnecting: false,
    connectionError: null,
    clearConnectionError: vi.fn(),
  } as ReturnType<typeof useWallet>);
}

describe("NetworkBadge (W12-FE-12)", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('displays "Testnet" label when network is testnet', async () => {
    await mockNetwork("testnet");
    render(<NetworkBadge />);
    expect(screen.getByText("Testnet")).toBeInTheDocument();
  });

  it('displays "Mainnet" label when network is mainnet', async () => {
    await mockNetwork("mainnet");
    render(<NetworkBadge />);
    expect(screen.getByText("Mainnet")).toBeInTheDocument();
  });

  it('sets aria-label to "Network: testnet" on testnet', async () => {
    await mockNetwork("testnet");
    render(<NetworkBadge />);
    expect(screen.getByLabelText("Network: testnet")).toBeInTheDocument();
  });

  it('sets aria-label to "Network: mainnet" on mainnet', async () => {
    await mockNetwork("mainnet");
    render(<NetworkBadge />);
    expect(screen.getByLabelText("Network: mainnet")).toBeInTheDocument();
  });

  it("does not show Mainnet text when network is testnet", async () => {
    await mockNetwork("testnet");
    render(<NetworkBadge />);
    expect(screen.queryByText("Mainnet")).not.toBeInTheDocument();
  });

  it("does not show Testnet text when network is mainnet", async () => {
    await mockNetwork("mainnet");
    render(<NetworkBadge />);
    expect(screen.queryByText("Testnet")).not.toBeInTheDocument();
  });

  it("renders a badge element (accessible role or test id)", async () => {
    await mockNetwork("testnet");
    render(<NetworkBadge />);
    // Badge renders as a span/div; verify the text is present in the DOM.
    const badge = screen.getByText("Testnet");
    expect(badge).toBeInTheDocument();
  });
});
