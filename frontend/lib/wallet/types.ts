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
