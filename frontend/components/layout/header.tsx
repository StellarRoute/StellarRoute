"use client"

import * as React from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { Menu } from "lucide-react"

import { Button } from "@/components/ui/button"
import { WalletButton } from "@/components/shared/wallet-button"
import { NetworkBadge } from "@/components/shared/network-badge"
import { MobileNav } from "./mobile-nav"
import { cn } from "@/lib/utils"
import { ThemeToggle } from "../ThemeToggle"

interface NavItem {
  label: string
  href: string
  disabled?: boolean
}

const navItems: NavItem[] = [
  { label: "Swap", href: "/" },
  { label: "Orderbook", href: "/orderbook" },
  { label: "History", href: "/history" },
  // Future routes - disabled for now
  // { label: "Analytics", href: "/analytics", disabled: true },
  // { label: "Docs", href: "/docs", disabled: true },
]

/**
 * Main header/navbar component
 *
 * Features:
 * - Sticky/fixed header with backdrop blur
 * - Logo and wordmark
 * - Navigation links with active route indicator
 * - Wallet connect button
 * - Theme toggle
 * - Network indicator badge
 * - Mobile hamburger menu
 */
export function Header() {
  const pathname = usePathname()
  const [mobileMenuOpen, setMobileMenuOpen] = React.useState(false)

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto flex h-16 items-center justify-between px-4 sm:px-6 lg:px-8">
        {/* Logo and Navigation */}
        <div className="flex items-center gap-6 md:gap-8">
          <Link
            href="/"
            className="flex items-center gap-2 font-semibold text-xl hover:opacity-80 transition-opacity"
            aria-label="StellarRoute Home"
          >
            <span className="text-primary">StellarRoute</span>
          </Link>

          {/* Desktop Navigation */}
          <nav className="hidden md:flex items-center gap-1" aria-label="Main navigation">
            {navItems.map((item) => {
              const isActive = pathname === item.href
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={cn(
                    "px-3 py-2 text-sm font-medium transition-colors rounded-md",
                    "hover:bg-accent hover:text-accent-foreground",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                    isActive
                      ? "bg-accent text-accent-foreground underline decoration-2 underline-offset-4"
                      : "text-muted-foreground",
                    item.disabled && "opacity-50 cursor-not-allowed pointer-events-none"
                  )}
                  aria-current={isActive ? "page" : undefined}
                  aria-disabled={item.disabled}
                >
                  {item.label}
                </Link>
              )
            })}
          </nav>
        </div>

        {/* Right Side Actions */}
        <div className="flex items-center gap-2">
          <div className="max-sm:hidden">
            <NetworkBadge />
            <ThemeToggle />
          </div>
          <div className="hidden md:block">
            <WalletButton />
          </div>

          {/* Mobile Menu Button */}
          <Button
            variant="ghost"
            size="icon"
            className="md:hidden h-11 w-11 flex items-center justify-center"
            onClick={() => setMobileMenuOpen(true)}
            aria-label="Open mobile menu"
            aria-expanded={mobileMenuOpen}
          >
            <Menu className="h-5 w-5" />
          </Button>
        </div>
      </div>

      {/* Mobile Navigation */}
      <MobileNav
        open={mobileMenuOpen}
        onOpenChange={setMobileMenuOpen}
        navItems={navItems}
        pathname={pathname}
      />
    </header>
  )
}
