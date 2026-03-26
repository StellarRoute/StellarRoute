"use client"

import * as React from "react"
import { usePathname } from "next/navigation"

import { Header } from "./header"
import { Footer } from "./footer"
import { cn } from "@/lib/utils"

interface AppShellProps {
  children: React.ReactNode
}

/**
 * Application shell component that wraps all pages
 *
 * Features:
 * - Consistent layout structure across all pages
 * - Header and footer on all pages
 * - Responsive content area with appropriate max-width
 * - Centered content for swap-type pages
 * - Full-width content for orderbook/analytics pages
 * - Consistent spacing and padding system
 */
export function AppShell({ children }: AppShellProps) {
  const pathname = usePathname()

  // Determine if page should be full-width (orderbook, analytics) or centered (swap)
  const isFullWidth = pathname?.startsWith("/orderbook") || pathname?.startsWith("/analytics")

  return (
    <div className="flex min-h-screen flex-col">
      <Header />

      <main
        className={cn(
          "flex-1",
          isFullWidth
            ? "w-full"
            : "container mx-auto w-full max-w-7xl px-3 py-8 sm:px-6 lg:px-8"
        )}
      >
        {children}
      </main>

      <Footer />
    </div>
  )
}
