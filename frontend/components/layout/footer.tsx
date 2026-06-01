"use client"

import Link from "next/link"
import { ExternalLink } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { useWallet } from "@/components/providers/wallet-provider"
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

/**
 * Footer component
 *
 * Features:
 * - Links to GitHub, Docs, Stellar.org, Community
 * - "Built for Stellar" branding
 * - Testnet/Mainnet indicator
 * - Minimal design that doesn't distract from main content
 */
export function Footer() {
  const { network } = useWallet()

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
            <Badge
              variant={network === "mainnet" ? "default" : "secondary"}
              className="w-fit"
              aria-label={`Network: ${network}`}
            >
              {network === "mainnet" ? "Mainnet" : "Testnet"}
            </Badge>
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
