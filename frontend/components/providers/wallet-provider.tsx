"use client"

import * as React from "react"

interface WalletState {
  address: string | null
  isConnected: boolean
  network: "testnet" | "mainnet"
}

interface WalletContextValue {
  address: string | null
  isConnected: boolean
  network: "testnet" | "mainnet"
  connect: () => Promise<void>
  disconnect: () => void
  setNetwork: (network: "testnet" | "mainnet") => void
  /**
   * Spendable balance for the sell asset (normalized decimal string), or null when
   * disconnected. Stub until wallet + Horizon integration; used by swap Max.
   */
  stubSpendableBalance: string | null
}

const WalletContext = React.createContext<WalletContextValue | undefined>(
  undefined
)

interface WalletProviderProps {
  children: React.ReactNode
  defaultNetwork?: "testnet" | "mainnet"
}

/**
 * Wallet provider for managing wallet connection state
 *
 * @remarks
 * This is a placeholder implementation. In the future, this will integrate
 * with Stellar wallet connectors (Freighter, XBull, etc.)
 */
export function WalletProvider({
  children,
  defaultNetwork = "testnet",
}: WalletProviderProps) {
  const [state, setState] = React.useState<WalletState>({
    address: null,
    isConnected: false,
    network: defaultNetwork,
  })

  const connect = React.useCallback(async () => {
    // Placeholder: Future implementation will integrate with wallet connectors
    // For now, simulate connection with a mock address
    setState({
      address: "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
      isConnected: true,
      network: state.network,
    })
  }, [state.network])

  const disconnect = React.useCallback(() => {
    setState({
      address: null,
      isConnected: false,
      network: state.network,
    })
  }, [state.network])

  const setNetwork = React.useCallback((network: "testnet" | "mainnet") => {
    setState((prev) => ({
      ...prev,
      network,
    }))
  }, [])

  const stubSpendableBalance = state.isConnected
    ? "10000.0000000"
    : null

  const value: WalletContextValue = {
    address: state.address,
    isConnected: state.isConnected,
    network: state.network,
    connect,
    disconnect,
    setNetwork,
    stubSpendableBalance,
  }

  return (
    <WalletContext.Provider value={value}>{children}</WalletContext.Provider>
  )
}

/**
 * Hook to access wallet context
 *
 * @throws Error if used outside WalletProvider
 */
export function useWallet() {
  const context = React.useContext(WalletContext)
  if (context === undefined) {
    throw new Error("useWallet must be used within a WalletProvider")
  }
  return context
}
