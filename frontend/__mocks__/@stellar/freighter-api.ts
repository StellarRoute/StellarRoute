import { vi } from "vitest";

let mockNetwork = "testnet";
let mockAddress = "GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

export function __setMockNetwork(network: string) {
  mockNetwork = network;
}

export function __setMockAddress(address: string) {
  mockAddress = address;
}

export const isAllowed = vi.fn().mockResolvedValue({ isAllowed: true });
export const requestAccess = vi.fn().mockImplementation(async () => ({ address: mockAddress }));
export const getAddress = vi.fn().mockImplementation(async () => ({ address: mockAddress }));
export const getNetworkDetails = vi.fn().mockImplementation(async () => ({
  network: mockNetwork,
  networkUrl:
    mockNetwork === "mainnet"
      ? "https://horizon.stellar.org"
      : "https://horizon-testnet.stellar.org",
  networkPassphrase:
    mockNetwork === "mainnet"
      ? "Public Global Stellar Network ; September 2015"
      : "Test SDF Network ; September 2015",
}));
export const signTransaction = vi.fn().mockResolvedValue({
  signedTxXdr: "",
  signerAddress: "",
});
export const isConnected = vi.fn().mockResolvedValue({ isConnected: true });
export const getNetwork = vi.fn().mockImplementation(async () => ({
  network: mockNetwork,
  networkPassphrase:
    mockNetwork === "mainnet"
      ? "Public Global Stellar Network ; September 2015"
      : "Test SDF Network ; September 2015",
}));
export const setAllowed = vi.fn().mockResolvedValue({ isAllowed: true });
export const WatchWalletChanges = vi.fn();
