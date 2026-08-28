import { SwapPageClient } from "./SwapPageClient";
import { buildPageMetadata } from "@/lib/seo";

export const metadata = buildPageMetadata({
  title: "Swap on Stellar DEX & Cross-Chain",
  description:
    "Swap on the Stellar DEX and Soroban AMMs with best-price routing, or run a cross-chain USDC swap via Circle CCTP — non-custodial.",
  path: "/swap",
});

export default function SwapPage() {
  return (
    <div className="mx-auto w-full max-w-5xl py-2 sm:py-4">
      <SwapPageClient />
      <div className="mt-10 flex flex-wrap justify-center gap-6 text-muted-foreground sm:gap-8">
        <div className="flex items-center gap-2">
          <div className="h-1.5 w-1.5 rounded-full bg-success" />
          <span className="font-mono text-[10px] font-semibold uppercase tracking-[0.2em]">
            Horizon live
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="h-1.5 w-1.5 rounded-full bg-chart-3" />
          <span className="font-mono text-[10px] font-semibold uppercase tracking-[0.2em]">
            Soroban ready
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="h-1.5 w-1.5 rounded-full bg-primary" />
          <span className="font-mono text-[10px] font-semibold uppercase tracking-[0.2em]">
            Best execution
          </span>
        </div>
      </div>
    </div>
  );
}
