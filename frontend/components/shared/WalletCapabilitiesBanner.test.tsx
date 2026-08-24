import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WalletProvider, useWallet } from "../providers/wallet-provider";
import { WalletCapabilitiesBanner } from "./WalletCapabilitiesBanner";

import * as freighter from "@stellar/freighter-api";

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

function WalletConsumer() {
  const { isConnected, capabilities, connect, disconnect } = useWallet();

  return (
    <div>
      <WalletCapabilitiesBanner />
      <span data-testid="connected">{String(isConnected)}</span>
      <span data-testid="canSign">{String(capabilities?.canSign ?? false)}</span>
      <button onClick={() => connect("freighter")}>Connect Freighter</button>
      <button onClick={() => connect("xbull")}>Connect xBull</button>
      <button onClick={disconnect}>Disconnect</button>
    </div>
  );
}

function renderComponent() {
  return render(
    <WalletProvider defaultNetwork="testnet">
      <WalletConsumer />
    </WalletProvider>
  );
}

describe("WalletCapabilities", () => {
  it("does not display banner when disconnected", () => {
    renderComponent();
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("does not display banner when wallet has full signing and network capabilities", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const user = userEvent.setup();
    renderComponent();

    await user.click(screen.getByRole("button", { name: "Connect Freighter" }));

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });
    expect(screen.getByTestId("canSign").textContent).toBe("true");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("displays banner warning when wallet lacks signing capability", async () => {
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn().mockResolvedValue({ publicKey: "GXBULLTESTADDRESS123" }),
    };

    const user = userEvent.setup();
    renderComponent();

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
});