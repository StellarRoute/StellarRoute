"use client"

import * as React from "react"
import { Copy, LogOut, Wallet } from "lucide-react"

import { Button } from "@/components/ui/button"
import { useWallet } from "@/components/providers/wallet-provider"
import { shortenAddress } from "@/lib/wallet"

export function WalletButton() {
  const {
    address,
    isConnected,
    connect,
    disconnect,
    availableWallets,
    isLoading,
    error,
    network,
    walletNetwork,
    networkMismatch,
    walletId,
  } = useWallet()

  const [copied, setCopied] = React.useState(false)

  const handleCopy = async () => {
    if (!address) return
    await navigator.clipboard.writeText(address)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1500)
  }

  if (isConnected && address) {
    return (
      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            onClick={() => void handleCopy()}
            className="gap-2"
            aria-label="Copy wallet address"
          >
            <Copy className="h-4 w-4" />
            <span>{copied ? "Copied" : shortenAddress(address)}</span>
          </Button>

          <Button
            variant="outline"
            onClick={disconnect}
            className="gap-2"
            aria-label="Disconnect wallet"
          >
            <LogOut className="h-4 w-4" />
            <span className="hidden sm:inline">Disconnect</span>
          </Button>
        </div>

        <div className="text-xs text-muted-foreground">
          {walletId ? `${walletId} connected` : "Wallet connected"} • App: {network}
          {" • "}Wallet: {walletNetwork ?? "unknown"}
        </div>

        {networkMismatch && (
          <div className="text-xs text-yellow-600">
            Wallet network does not match the app network.
          </div>
        )}

        {error && <div className="text-xs text-red-500">{error.message}</div>}
      </div>
    )
  }

  const noWalletInstalled =
    availableWallets.length > 0 &&
    availableWallets.every((wallet) => !wallet.installed)

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        {availableWallets.map((wallet) => (
          <Button
            key={wallet.id}
            onClick={() => void connect(wallet.id)}
            disabled={isLoading || !wallet.installed}
            variant={wallet.installed ? "default" : "outline"}
            className="gap-2"
            aria-label={`Connect ${wallet.label}`}
          >
            <Wallet className="h-4 w-4" />
            <span>{wallet.label}</span>
          </Button>
        ))}
      </div>

      {noWalletInstalled && (
        <div className="text-xs text-muted-foreground">
          No supported wallet found. Install{" "}
          {availableWallets.map((wallet, index) => (
            <React.Fragment key={wallet.id}>
              {index > 0 ? " or " : ""}
              <a
                href={wallet.installUrl}
                target="_blank"
                rel="noreferrer"
                className="underline"
              >
                {wallet.label}
              </a>
            </React.Fragment>
          ))}
          .
        </div>
      )}

      {error && <div className="text-xs text-red-500">{error.message}</div>}
    </div>
  )
}