export type SupportedWallet = "freighter" | "xbull";

export type WalletNetwork = "testnet" | "mainnet" | "futurenet" | string;

export type WalletSession = {
  walletId: SupportedWallet | null;
  address: string | null;
  network: WalletNetwork | null;
  isConnected: boolean;
};

export type AvailableWallet = {
  id: SupportedWallet;
  label: string;
  installed: boolean;
};

export type WalletError = {
  message: string;
  code?: string;
};

export type AccountSwitchState = {
  isDetecting: boolean;
  hasChanged: boolean;
  previousAddress: string | null;
};

export type Capability =
  | "sign_transaction"
  | "view_address"
  | "view_network"
  | "request_access";

export type CapabilityStatus = {
  capability: Capability;
  allowed: boolean;
  reason?: string;
  resolution?: string;
};

export type Capabilities = {
  checkedAt: number;
  statuses: CapabilityStatus[];
};
