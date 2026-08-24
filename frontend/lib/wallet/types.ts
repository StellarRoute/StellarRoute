/**
 * Stellar browser wallets used by WalletProvider / swap UI.
 * Multi-chain adapters (EVM, Solana, …) live under `./adapters` and use
 * string adapter ids + `ChainFamily` instead of extending this union.
 */
export type SupportedWallet = 'freighter' | 'xbull' | 'albedo' | 'lobstr';

export type WalletNetwork = 'testnet' | 'mainnet' | 'futurenet' | string;

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

export interface WalletCapabilities {
  canSign: boolean;
  supportedNetworks: WalletNetwork[];
  supportsNetworkSwitching: boolean;
}

export interface WalletCapabilityStatus {
  canSign: boolean;
  networkSupported: boolean;
  missingCapabilities: string[];
}
