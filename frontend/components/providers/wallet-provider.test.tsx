import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WalletProvider, useWallet } from "./wallet-provider";
import { WalletCapabilitiesBanner } from "@/components/shared/WalletCapabilitiesBanner";

import * as freighter from "@stellar/freighter-api";

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

// ── Test consumer ──────────────────────────────────────────────────────────────
function WalletConsumer() {
  const {
    address,
    isConnected,
    network,
    walletId,
    error,
    isLoading,
    capabilities,
    networkMismatch,
    stubSpendableBalance,
    connect,
    disconnect,
  } = useWallet();

  return (
    <div>
      <WalletCapabilitiesBanner />
      <span data-testid="connected">{String(isConnected)}</span>
      <span data-testid="address">{address ?? "none"}</span>
      <span data-testid="network">{network}</span>
      <span data-testid="walletId">{walletId ?? "none"}</span>
      <span data-testid="canSign">{String(capabilities?.canSign ?? false)}</span>
      <span data-testid="error">{error?.message ?? "none"}</span>
      <span data-testid="loading">{String(isLoading)}</span>
      <span data-testid="mismatch">{String(networkMismatch)}</span>
      <span data-testid="balance">{stubSpendableBalance ?? "none"}</span>
      <button onClick={() => connect("freighter")}>Connect Freighter</button>
      <button onClick={() => connect("xbull")}>Connect xBull</button>
      <button onClick={disconnect}>Disconnect</button>
    </div>
  );
}

function renderWithProvider(defaultNetwork?: "testnet" | "mainnet") {
  return render(
    <WalletProvider defaultNetwork={defaultNetwork ?? "testnet"}>
      <WalletConsumer />
    </WalletProvider>
  );
}

// ── Tests ──────────────────────────────────────────────────────────────────────
describe("WalletCapabilities & Provider", () => {
  it("provides disconnected state by default", () => {
    renderWithProvider();
    expect(screen.getByTestId("connected").textContent).toBe("false");
    expect(screen.getByTestId("address").textContent).toBe("none");
    expect(screen.getByTestId("network").textContent).toBe("testnet");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("connects Freighter with full capabilities", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole("button", { name: "Connect Freighter" }));

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });
    expect(screen.getByTestId("canSign").textContent).toBe("true");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("warns about missing sign capability when wallet lacks signing support", async () => {
    // Mock window.xbull for connectWallet
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn().mockResolvedValue({ publicKey: "GXBULLTESTADDRESS123" }),
    };

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole("button", { name: "Connect xBull" }));

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });

    expect(screen.getByTestId("canSign").textContent).toBe("false");
    expect(screen.getByTestId("wallet-capabilities-banner")).toBeDefined();
    expect(screen.getByTestId("capability-warning").textContent).toContain(
      "Transaction signing is not supported"
    );
  });

  it("disconnects and clears state", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole("button", { name: "Connect Freighter" }));
    await waitFor(() => expect(screen.getByTestId("connected").textContent).toBe("true"));

    await user.click(screen.getByRole("button", { name: "Disconnect" }));

    expect(screen.getByTestId("connected").textContent).toBe("false");
    expect(screen.getByTestId("address").textContent).toBe("none");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });
});