import { buildPageMetadata } from "@/lib/seo";
import { GuidePageClient } from "./GuidePageClient";

export const metadata = buildPageMetadata({
  title: "First Stellar DEX Swap Guide",
  description:
    "Step-by-step guide for your first Stellar DEX swap on StellarRoute: wallet, trustline, slippage, and confirm.",
  path: "/guide",
});

export default function FirstSwapGuidePage() {
  return <GuidePageClient />;
}
