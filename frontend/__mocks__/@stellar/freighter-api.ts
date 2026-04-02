import { vi } from "vitest";

export const isAllowed = vi.fn().mockResolvedValue({ isAllowed: false });
export const requestAccess = vi.fn().mockResolvedValue({ address: "" });
export const getAddress = vi.fn().mockResolvedValue({ address: "" });
export const getNetworkDetails = vi.fn().mockResolvedValue({
  network: "testnet",
  networkUrl: "https://horizon-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
});
export const signTransaction = vi.fn().mockResolvedValue({
  signedTxXdr: "",
  signerAddress: "",
});
export const isConnected = vi.fn().mockResolvedValue({ isConnected: false });
export const getNetwork = vi.fn().mockResolvedValue({
  network: "testnet",
  networkPassphrase: "Test SDF Network ; September 2015",
});
export const setAllowed = vi.fn().mockResolvedValue({ isAllowed: true });
export const WatchWalletChanges = vi.fn();
