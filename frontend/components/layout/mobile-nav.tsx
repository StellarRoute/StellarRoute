"use client"

import * as React from "react"
import Link from "next/link"

import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
//import { WalletButton } from "@/components/shared/WalletButton"
import { NetworkBadge } from "@/components/shared/network-badge"
import { cn } from "@/lib/utils"
import { ThemeToggle } from "../ThemeToggle"

interface NavItem {
  label: string
  href: string
  disabled?: boolean
}

interface MobileNavProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  navItems: NavItem[]
  pathname: string
}

/**
 * Mobile navigation drawer component
 *
 * Features:
 * - Slide-in drawer from right on mobile
 * - Navigation links stacked vertically
 * - Wallet connect and theme toggle accessible
 * - Closes on route change and outside click
 * - Smooth animations
 */
export function MobileNav({
  open,
  onOpenChange,
  navItems,
  pathname,
}: MobileNavProps) {
  const handleLinkClick = () => {
    onOpenChange(false)
  }

  // Close mobile menu when pathname changes
  React.useEffect(() => {
    if (open) {
      onOpenChange(false)
    }
  }, [pathname]) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-[300px] sm:w-[400px]">
        <SheetHeader>
          <SheetTitle className="text-left">Menu</SheetTitle>
        </SheetHeader>

        <nav className="mt-8 flex flex-col gap-4" aria-label="Mobile navigation">
          {navItems.map((item) => {
            const isActive = pathname === item.href
            return (
              <Link
                key={item.href}
                href={item.href}
                onClick={handleLinkClick}
                className={cn(
                  "px-4 py-3 text-base font-medium transition-colors rounded-md",
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

        {/* Mobile Actions */}
        <div className="mt-8 flex flex-col gap-4 border-t pt-6">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Network</span>
            <NetworkBadge />
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Theme</span>
            <ThemeToggle />
          </div>
          <div className="pt-2">
           {/* <WalletButton /> */}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
