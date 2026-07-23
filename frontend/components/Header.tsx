"use client";

import Link from "next/link";
import { ThemeToggle } from "./ThemeToggle";
import { WalletButton } from "@/components/shared/wallet-button";

export function Header() {
  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
      <div className="container flex h-14 items-center mx-auto px-4">
        <div className="mr-4 flex">
          <Link href="/" className="mr-6 flex items-center space-x-2">
            <span className="hidden font-bold sm:inline-block">
              StellarRoute
            </span>
          </Link>
        </div>

        <div className="flex flex-1 items-center justify-between space-x-2 md:justify-end">
          <div className="w-full flex-1 md:w-auto md:flex-none">
            {/* future nav/search area */}
          </div>

          <nav className="flex items-center gap-4">
            <Link
              href="/history"
              className="text-sm font-medium hover:text-primary transition-colors"
            >
              History
            </Link>

            <WalletButton />
            <ThemeToggle />
          </nav>
        </div>
      </div>
    </header>
  );
}