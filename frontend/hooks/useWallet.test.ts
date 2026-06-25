import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useWallet } from "./useWallet";
import { WalletProvider } from "@/components/providers/wallet-provider";

// The @stellar/freighter-api mock is wired via vitest.config.ts alias
import * as freighter from "@stellar/freighter-api";

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
});

function renderWalletHook() {
  return renderHook(() => useWallet(), {
    wrapper: WalletProvider,
  });
}

describe("useWallet – initial state", () => {
  it("starts disconnected with no address", () => {
    const { result } = renderWalletHook();
    expect(result.current.session.isConnected).toBe(false);
    expect(result.current.session.address).toBeNull();
    expect(result.current.session.walletId).toBeNull();
  });

  it("shortAddress is empty when not connected", () => {
    const { result } = renderWalletHook();
    expect(result.current.shortAddress).toBe("");
  });
});

describe("useWallet – connect (Freighter)", () => {
  it("sets session on successful connect", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "https://horizon-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
    });

    const { result } = renderWalletHook();

    await act(async () => {
      await result.current.connect("freighter");
    });

    expect(result.current.session.isConnected).toBe(true);
    expect(result.current.session.address).toBe("GABCDEFGHIJKLMNOPWXYZ");
    expect(result.current.session.walletId).toBe("freighter");
    expect(result.current.session.network).toBe("testnet");
  });

  it("truncates address to GABC...WXYZ format", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const { result } = renderWalletHook();

    await act(async () => {
      await result.current.connect("freighter");
    });

    expect(result.current.shortAddress).toBe("GABC...WXYZ");
  });

  it("sets error message on rejected connection", async () => {
    vi.mocked(freighter.requestAccess).mockRejectedValueOnce(new Error("User rejected"));

    const { result } = renderWalletHook();

    await act(async () => {
      await result.current.connect("freighter");
    });

    expect(result.current.session.isConnected).toBe(false);
    expect(result.current.error).toContain("reject");
  });

  it("sets error when wallet is locked", async () => {
    vi.mocked(freighter.requestAccess).mockRejectedValueOnce(new Error("Wallet is locked"));

    const { result } = renderWalletHook();

    await act(async () => {
      await result.current.connect("freighter");
    });

    expect(result.current.session.isConnected).toBe(false);
    expect(result.current.error).toContain("locked");
  });
});

describe("useWallet – disconnect", () => {
  it("clears session on disconnect", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const { result } = renderWalletHook();

    await act(async () => {
      await result.current.connect("freighter");
    });

    expect(result.current.session.isConnected).toBe(true);

    act(() => {
      result.current.disconnect();
    });

    expect(result.current.session.isConnected).toBe(false);
    expect(result.current.session.address).toBeNull();
    expect(result.current.session.walletId).toBeNull();
    expect(result.current.error).toBeNull();
  });
});
