import Link from "next/link";
import { ExternalLink } from "lucide-react";

import { buildPageMetadata } from "@/lib/seo";
import { CARD_ENABLED } from "@/lib/constants";
import AgentGuideSection from "./agent-guide-section";

export const metadata = buildPageMetadata({
  title: "First Stellar DEX Swap Guide",
  description:
    "Step-by-step guide for your first Stellar DEX swap on StellarRoute: wallet, trustline, slippage, and confirm.",
  path: "/guide",
});

const steps = [
  {
    id: "step-connect-wallet",
    title: "Connect your wallet",
    body: "Use Freighter or xBull. Match the network badge in the footer (Testnet vs Mainnet) before you trade.",
    image: "/guide/01-connect.svg",
    alt: "Wallet connect screen showing Freighter and xBull options with network badge",
  },
  {
    id: "step-fund-reserve",
    title: "Fund and reserve XLM",
    body: "Keep enough XLM for network fees and Stellar base reserves. On testnet, Friendbot can fund a new account.",
    image: "/guide/02-fund.svg",
    alt: "Account balance panel showing XLM balance and base reserve requirement",
  },
  {
    id: "step-add-trustline",
    title: "Add a trustline if needed",
    body: "Non-XLM receive assets usually need a trustline. Approve the trustline transaction in your wallet when prompted.",
    image: "/guide/03-trustline.svg",
    alt: "Trustline approval prompt in the wallet for the receive asset",
  },
  {
    id: "step-pick-pair",
    title: "Pick a pair and enter a small amount",
    body: "Choose pay/receive assets, enter a modest size for your first live swap, and wait for the best-route quote.",
    image: "/guide/04-pair.svg",
    alt: "Swap card with pay and receive assets selected and a best-route quote displayed",
  },
  {
    id: "step-set-slippage",
    title: "Set slippage and review the route",
    body: "Start near 0.5% slippage unless markets are moving quickly. Read high-impact warnings before confirming.",
    image: "/guide/05-slippage.svg",
    alt: "Slippage selector set to 0.5% with route details and impact level shown",
  },
  {
    id: "step-confirm-wallet",
    title: "Confirm in your wallet",
    body: "Review amounts in the wallet prompt, sign, then track status in the app. StellarRoute never holds your keys.",
    image: "/guide/06-confirm.svg",
    alt: "Wallet confirmation popup showing PathPaymentStrictSend with amounts and fee",
  },
] as const;

export default function FirstSwapGuidePage() {
  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-10 sm:px-6 lg:px-8">
      {/* Skip link: lets keyboard users jump straight to the step list */}
      <a
        href="#guide-steps"
        className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:ring-2 focus:ring-ring focus:ring-offset-2"
      >
        Skip to steps
      </a>

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

          {/* Table of contents */}
          <nav aria-label="Guide table of contents" className="pt-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Steps
            </p>
            <ol className="space-y-1 text-sm" data-testid="guide-toc">
              {steps.map((step, index) => (
                <li key={step.id}>
                  <a
                    href={`#${step.id}`}
                    className="inline-flex items-center gap-1.5 text-muted-foreground underline-offset-4 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 rounded-sm"
                  >
                    <span className="tabular-nums text-xs font-medium w-4 shrink-0">
                      {index + 1}.
                    </span>
                    {step.title}
                  </a>
                </li>
              ))}
            </ol>
          </nav>
        </header>

        <ol id="guide-steps" className="space-y-4">
          {steps.map((step, index) => (
            <li
              key={step.title}
              id={step.id}
              className="rounded-xl border bg-card p-5 text-card-foreground scroll-mt-4"
            >
              <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Step {index + 1}
              </p>
              <h2 className="mt-1 text-lg font-semibold">{step.title}</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                {step.body}
              </p>
              <img
                src={step.image}
                alt={step.alt}
                width={640}
                height={360}
                loading="lazy"
                className="mt-4 w-full rounded-lg border"
              />
            </li>
          ))}
        </ol>

        <AgentGuideSection />

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

        {CARD_ENABLED && (
          <aside className="rounded-xl border border-primary/30 bg-primary/5 p-5 text-sm text-primary-foreground">
            <p className="font-medium">Card payments (beta)</p>
            <p className="mt-2 leading-6">
              Pay with a card: USDC balance, fiat charge, confirm in the wallet.
            </p>
          </aside>
        )}
      </div>
    </main>
  );
}
