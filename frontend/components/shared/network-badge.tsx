"use client"

import { Badge } from "@/components/ui/badge"
import { useWallet } from "@/components/providers/wallet-provider"

/**
 * Network indicator badge component
 *
 * Displays the current network (Testnet/Mainnet)
 */
export function NetworkBadge() {
  const { network } = useWallet()

  return (
    <Badge
      variant={network === "mainnet" ? "default" : "secondary"}
      className="inline-flex"
      aria-label={`Network: ${network}`}
    >
      {network === "mainnet" ? "Mainnet" : "Testnet"}
    </Badge>
  )
}
