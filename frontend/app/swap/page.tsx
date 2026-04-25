import { SwapCard } from "@/components/swap/SwapCard";
import { Metadata } from "next";

export const metadata: Metadata = {
  title: "Swap Tokens | StellarRoute",
  description: "Swap assets on Stellar with the best rates and lowest slippage across all DEXs and AMMs.",
  openGraph: {
    title: "Swap Tokens | StellarRoute",
    description: "Best-price routing across Stellar DEX and Soroban AMM pools.",
    type: "website",
  },
};

export default function SwapPage() {
  return (
    <main className="min-h-[calc(100vh-80px)] flex items-center justify-center py-10 px-4 sm:px-6 lg:px-8 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-primary/5 via-background to-background">
      <div className="w-full flex flex-col items-center">
        {/* Hero Section / Title */}
        <div className="text-center mb-8 space-y-2">
          <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-foreground">
            Universal Swap
          </h1>
          <p className="text-muted-foreground text-sm font-medium">
            Optimal pathfinding across SDEX and Soroban AMMs
          </p>
        </div>

        {/* The Swap Card */}
        <SwapCard />

        {/* Extra Info / Social Proof */}
        <div className="mt-12 flex flex-wrap justify-center gap-8 opacity-50 grayscale hover:grayscale-0 transition-all duration-500">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-emerald-500" />
            <span className="text-xs font-bold uppercase tracking-widest">Horizon Live</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-blue-500" />
            <span className="text-xs font-bold uppercase tracking-widest">Soroban Ready</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-primary" />
            <span className="text-xs font-bold uppercase tracking-widest">Best Execution</span>
          </div>
        </div>
      </div>
    </main>
  );
}
