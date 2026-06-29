"use client"

import Link from "next/link"
import { ExternalLink } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { useWallet } from "@/components/providers/wallet-provider"
import { getAllowedNetworks, type AppNetwork } from "@/lib/network-policy"
import { cn } from "@/lib/utils"

interface FooterLink {
  label: string
  href: string
  external?: boolean
}

const footerLinks: FooterLink[] = [
  { label: "Status", href: "/status", external: false },
  { label: "GitHub", href: "https://github.com/stellarroute/stellarroute", external: true },
  { label: "Docs", href: "/docs", external: false },
  { label: "Stellar.org", href: "https://www.stellar.org", external: true },
  { label: "Community", href: "https://discord.gg/stellar", external: true },
]

const NETWORK_LABELS: Record<AppNetwork, string> = {
  testnet: "Testnet",
  mainnet: "Mainnet",
}

function FooterNetworkSwitch() {
  const { network, setNetwork } = useWallet()
  const allowedNetworks = getAllowedNetworks()
  const currentNetwork =
    allowedNetworks.find((value) => value === network) ?? allowedNetworks[0]

  if (allowedNetworks.length <= 1) {
    return (
      <Badge
        variant={currentNetwork === "mainnet" ? "default" : "secondary"}
        className="w-fit"
        aria-label={`Network: ${currentNetwork}`}
      >
        {NETWORK_LABELS[currentNetwork]}
      </Badge>
    )
  }

  return (
    <label className="flex flex-col gap-1">
      <span className="sr-only">Stellar network</span>
      <select
        aria-label="Stellar network"
        value={currentNetwork}
        onChange={(event) => setNetwork(event.target.value)}
        className={cn(
          "h-9 rounded-md border border-input bg-background px-3 py-1 text-sm font-medium",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        )}
      >
        {allowedNetworks.map((value) => (
          <option key={value} value={value}>
            {NETWORK_LABELS[value]}
          </option>
        ))}
      </select>
    </label>
  )
}

/**
 * Footer component
 *
 * Features:
 * - Links to GitHub, Docs, Stellar.org, Community
 * - "Built for Stellar" branding
 * - Testnet/Mainnet switch (when policy allows)
 * - Minimal design that doesn't distract from main content
 */
export function Footer() {
  return (
    <footer className="border-t bg-background">
      <div className="container mx-auto px-4 py-8 sm:px-6 lg:px-8">
        <div className="flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
          {/* Links */}
          <nav
            className="flex flex-wrap items-center gap-4 sm:gap-6"
            aria-label="Footer navigation"
          >
            {footerLinks.map((link) => {
              const LinkComponent = link.external ? "a" : Link
              const linkProps = link.external
                ? { href: link.href, target: "_blank", rel: "noopener noreferrer" }
                : { href: link.href }

              return (
                <LinkComponent
                  key={link.href}
                  {...linkProps}
                  className={cn(
                    "text-sm text-muted-foreground hover:text-foreground transition-colors",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded-sm",
                    "inline-flex items-center gap-1"
                  )}
                >
                  {link.label}
                  {link.external && (
                    <ExternalLink className="h-3 w-3" aria-hidden="true" />
                  )}
                </LinkComponent>
              )
            })}
          </nav>

          {/* Branding and Network */}
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
            <FooterNetworkSwitch />
            <p className="text-sm text-muted-foreground">
              Built for{" "}
              <a
                href="https://www.stellar.org"
                target="_blank"
                rel="noopener noreferrer"
                className="font-medium hover:text-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded-sm"
              >
                Stellar
              </a>
            </p>
          </div>
        </div>
      </div>
    </footer>
  )
}
