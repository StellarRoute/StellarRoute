import Link from "next/link";
import { ExternalLink } from "lucide-react";

import { buildPageMetadata } from "@/lib/seo";

export const metadata = buildPageMetadata({
  title: "First Stellar DEX Swap Guide",
  description:
    "Step-by-step guide for your first Stellar DEX swap on StellarRoute: wallet, trustline, slippage, and confirm.",
  path: "/guide",
});

const steps = [
  {
    title: "Connect your wallet",
    body: "Use Freighter or xBull. Match the network badge in the footer (Testnet vs Mainnet) before you trade.",
  },
  {
    title: "Fund and reserve XLM",
    body: "Keep enough XLM for network fees and Stellar base reserves. On testnet, Friendbot can fund a new account.",
  },
  {
    title: "Add a trustline if needed",
    body: "Non-XLM receive assets usually need a trustline. Approve the trustline transaction in your wallet when prompted.",
  },
  {
    title: "Pick a pair and enter a small amount",
    body: "Choose pay/receive assets, enter a modest size for your first live swap, and wait for the best-route quote.",
  },
  {
    title: "Set slippage and review the route",
    body: "Start near 0.5% slippage unless markets are moving quickly. Read high-impact warnings before confirming.",
  },
  {
    title: "Confirm in your wallet",
    body: "Review amounts in the wallet prompt, sign, then track status in the app. StellarRoute never holds your keys.",
  },
] as const;

export default function FirstSwapGuidePage() {
  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-10 sm:px-6 lg:px-8">
      <div className="container mx-auto max-w-3xl space-y-8">
        <header className="space-y-3">
          <p className="text-sm font-medium uppercase text-muted-foreground">
            User guide
          </p>
          <h1 className="text-3xl font-extrabold tracking-tight sm:text-4xl">
            Your first live swap
          </h1>
          <p className="max-w-2xl text-lg text-muted-foreground">
            A short path for traders: wallet → trustline → quote → slippage →
            confirm. For the full annotated write-up, see the repository guide.
          </p>
          <div className="flex flex-wrap gap-3 pt-1">
            <Link
              href="/swap"
              className="inline-flex h-10 items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              Open swap
            </Link>
            <a
              href="https://github.com/StellarRoute/StellarRoute/blob/main/docs/user-guide-first-live-swap.md"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex h-10 items-center gap-1.5 rounded-md border px-4 text-sm font-medium hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              Full guide on GitHub
              <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
            </a>
          </div>
        </header>

        <ol className="space-y-4">
          {steps.map((step, index) => (
            <li
              key={step.title}
              className="rounded-xl border bg-card p-5 text-card-foreground"
            >
              <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Step {index + 1}
              </p>
              <h2 className="mt-1 text-lg font-semibold">{step.title}</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                {step.body}
              </p>
            </li>
          ))}
        </ol>

        <aside className="rounded-xl border border-dashed p-5 text-sm text-muted-foreground">
          <p className="font-medium text-foreground">Before you confirm</p>
          <p className="mt-2 leading-6">
            Aggregated routes can still slip, fail, or traverse multiple hops.
            Read the{" "}
            <a
              href="https://github.com/StellarRoute/StellarRoute/blob/main/docs/risk-disclosure.md"
              target="_blank"
              rel="noopener noreferrer"
              className="font-medium text-foreground underline-offset-4 hover:underline"
            >
              risk disclosure
            </a>{" "}
            and start with a small amount. Press{" "}
            <kbd className="rounded border bg-muted px-1.5 py-0.5 font-mono text-xs">
              ?
            </kbd>{" "}
            on the swap card anytime for shortcuts and a link back here.
          </p>
        </aside>
      </div>
    </main>
  );
}
