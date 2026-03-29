export type WalletNetwork = "testnet" | "mainnet"

export type SupportedWallet = "freighter" | "xbull"

export interface AvailableWallet {
  id: SupportedWallet
  label: string
  installed: boolean
  installUrl?: string
}

export interface WalletSession {
  walletId: SupportedWallet | null
  address: string | null
  network: WalletNetwork | null
  isConnected: boolean
}

export type WalletErrorCode =
  | "NO_WALLET"
  | "USER_REJECTED"
  | "WALLET_LOCKED"
  | "NETWORK_MISMATCH"
  | "UNKNOWN"

export interface WalletError {
  code: WalletErrorCode
  message: string
}