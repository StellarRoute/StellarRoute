import type {
  AvailableWallet,
  SupportedWallet,
  WalletError,
  WalletNetwork,
  WalletSession,
} from "./types"

declare global {
  interface Window {
    xBullSDK?: {
      connect?: () => Promise<{ publicKey?: string }>
      getNetwork?: () => Promise<string>
    }
  }
}

const FREIGHTER_INSTALL_URL = "https://www.freighter.app/"
const XBULL_INSTALL_URL = "https://xbull.app/"

function normalizeNetwork(network: string | null | undefined): WalletNetwork | null {
  if (!network) return null

  const value = network.toLowerCase()

  if (value.includes("testnet")) return "testnet"
  if (value.includes("mainnet") || value.includes("public")) return "mainnet"

  return null
}

function mapWalletError(message: string): WalletError {
  const lower = message.toLowerCase()

  if (lower.includes("reject") || lower.includes("denied")) {
    return {
      code: "USER_REJECTED",
      message: "Wallet connection request was rejected.",
    }
  }

  if (lower.includes("lock")) {
    return {
      code: "WALLET_LOCKED",
      message: "Wallet is locked. Please unlock it and try again.",
    }
  }

  if (
    lower.includes("not installed") ||
    lower.includes("not found") ||
    lower.includes("no wallet")
  ) {
    return {
      code: "NO_WALLET",
      message: "No supported wallet is installed.",
    }
  }

  return {
    code: "UNKNOWN",
    message,
  }
}

export async function getAvailableWallets(): Promise<AvailableWallet[]> {
  let freighterInstalled = false

  try {
    const freighter = await import("@stellar/freighter-api")
    const connection = await freighter.isConnected()
    freighterInstalled = connection.isConnected
  } catch {
    freighterInstalled = false
  }

  const xbullInstalled =
    typeof window !== "undefined" && typeof window.xBullSDK !== "undefined"

  return [
    {
      id: "freighter",
      label: "Freighter",
      installed: freighterInstalled,
      installUrl: FREIGHTER_INSTALL_URL,
    },
    {
      id: "xbull",
      label: "xBull",
      installed: xbullInstalled,
      installUrl: XBULL_INSTALL_URL,
    },
  ]
}

export async function connectWallet(
  walletId: SupportedWallet
): Promise<WalletSession> {
  if (walletId === "freighter") {
    try {
      const freighter = await import("@stellar/freighter-api")

      const installed = await freighter.isConnected()
      if (!installed) {
        throw new Error("Freighter not installed")
      }

      const addressResult = await freighter.getAddress()
      const address = addressResult.address
      const details =
        typeof freighter.getNetworkDetails === "function"
          ? await freighter.getNetworkDetails()
          : null

      return {
        walletId: "freighter",
        address,
        network: normalizeNetwork(details?.network),
        isConnected: true,
      }
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to connect Freighter wallet."
      throw mapWalletError(message)
    }
  }

  if (walletId === "xbull") {
    try {
      if (typeof window === "undefined" || !window.xBullSDK) {
        throw new Error("xBull not installed")
      }

      const result = await window.xBullSDK.connect?.()
      const rawNetwork = await window.xBullSDK.getNetwork?.()

      return {
        walletId: "xbull",
        address: result?.publicKey ?? null,
        network: normalizeNetwork(rawNetwork),
        isConnected: Boolean(result?.publicKey),
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to connect xBull wallet."
      throw mapWalletError(message)
    }
  }

  throw mapWalletError("Unsupported wallet")
}

export function disconnectWallet(): WalletSession {
  return {
    walletId: null,
    address: null,
    network: null,
    isConnected: false,
  }
}

export async function signTransactionStub(xdr: string): Promise<string> {
  void xdr
  throw new Error("signTransactionStub is not implemented yet.")

}

export function shortenAddress(address: string | null): string {
  if (!address) return ""
  if (address.length <= 10) return address
  return `${address.slice(0, 4)}...${address.slice(-5)}`
}