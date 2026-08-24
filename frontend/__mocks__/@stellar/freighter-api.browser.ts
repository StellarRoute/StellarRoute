const MOCK_ADDRESS =
  "GAKCEEZFGQ6HPTROFALCQXBEMJSF6DMZEHRYCGPTNJ5KLW3ZW7NXAEIO";
export interface FreighterMockConfig {
  signBehavior?: "resolve" | "reject";
  signedTxXdr?: string;
  rejectMessage?: string;
}

declare global {
  interface Window {
    __stellarrouteFreighterMock?: FreighterMockConfig;
  }
}

function getMockConfig(): FreighterMockConfig {
  if (typeof window !== "undefined" && window.__stellarrouteFreighterMock) {
    return window.__stellarrouteFreighterMock;
  }
  return {};
}

export const isAllowed = async () => ({ isAllowed: true });
export const requestAccess = async () => ({ address: MOCK_ADDRESS });
export const getAddress = async () => ({ address: MOCK_ADDRESS });
export const getNetworkDetails = async () => ({
  network: "testnet",
  networkUrl: "https://horizon-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
});
export const signTransaction = async (xdr: string) => {
  void xdr;
  const config = getMockConfig();
  if (config.signBehavior === "reject") {
    return {
      error: { message: config.rejectMessage ?? "User declined" },
    };
  }
  return {
    signedTxXdr: config.signedTxXdr ?? "AAAAmock_signed_xdr_e2e",
    signerAddress: MOCK_ADDRESS,
  };
};
export const isConnected = async () => ({ isConnected: true });
export const getNetwork = async () => ({
  network: "testnet",
  networkPassphrase: "Test SDF Network ; September 2015",
});
export const setAllowed = async () => ({ isAllowed: true });
export const WatchWalletChanges = () => {};
