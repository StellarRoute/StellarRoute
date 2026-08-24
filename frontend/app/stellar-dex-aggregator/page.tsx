import Link from "next/link";
import { ArrowRight } from "lucide-react";

import { JsonLd } from "@/components/seo/JsonLd";
import { Button } from "@/components/ui/button";
import { buildPageMetadata, faqJsonLd } from "@/lib/seo";

const faqs = [
  {
    question: "What is a Stellar DEX aggregator?",
    answer:
      "A Stellar DEX aggregator compares liquidity across Stellar trading venues — typically the classic SDEX order book and Soroban AMM pools — then helps you execute the best available route from one interface.",
  },
  {
    question: "Does StellarRoute replace the Stellar DEX?",
    answer:
      "No. StellarRoute routes across Stellar DEX (SDEX) offers and Soroban AMMs. Settlement still happens on Stellar through your wallet-signed transaction.",
  },
  {
    question: "Can I combine DEX aggregation with a cross-chain swap?",
    answer:
      "Yes. Use StellarRoute for best-price Stellar routing, then the cross-chain deck for Circle CCTP USDC between Stellar and Ethereum when you need another chain.",
  },
] as const;

export const metadata = buildPageMetadata({
  title: "Stellar DEX Aggregator",
  description:
    "Best-price Stellar DEX aggregator across SDEX order books and Soroban AMM pools. Non-custodial routing with optional cross-chain USDC via CCTP.",
  path: "/stellar-dex-aggregator",
});

export default function StellarDexAggregatorPage() {
  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-12 sm:px-6 lg:px-8">
      <JsonLd data={faqJsonLd(faqs)} />
      <div className="container mx-auto max-w-3xl space-y-12">
        <header className="space-y-4">
          <p className="font-mono text-xs font-medium uppercase tracking-[0.22em] text-primary">
            Stellar DEX aggregator
          </p>
          <h1 className="font-display text-4xl font-semibold tracking-[-0.045em] sm:text-5xl">
            Stellar DEX aggregator for SDEX and Soroban
          </h1>
          <p className="max-w-2xl text-lg leading-relaxed text-muted-foreground">
            StellarRoute compares executable liquidity across the Stellar DEX
            and Soroban AMMs so you can swap with better route awareness —
            non-custodial, wallet-signed, built for Stellar.
          </p>
          <div className="flex flex-wrap gap-3 pt-2">
            <Button asChild size="lg" className="h-11">
              <Link href="/swap">
                Open Stellar swap
                <ArrowRight className="h-4 w-4" aria-hidden="true" />
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg" className="h-11">
              <Link href="/cross-chain-swap">Cross-chain swap</Link>
            </Button>
          </div>
        </header>

        <section className="space-y-4" aria-labelledby="what-title">
          <h2 id="what-title" className="font-display text-2xl font-semibold">
            What a Stellar DEX aggregator actually does
          </h2>
          <p className="leading-relaxed text-muted-foreground">
            Stellar has multiple liquidity surfaces: classic Path Payment /
            SDEX offers and newer Soroban AMM pools. A DEX aggregator indexes
            those venues, filters stale or unhealthy books, and ranks executable
            routes — so you are not guessing which single pool is best.
          </p>
        </section>

        <section className="space-y-4" aria-labelledby="features-title">
          <h2 id="features-title" className="font-display text-2xl font-semibold">
            Built for Stellar traders
          </h2>
          <ul className="space-y-3 leading-relaxed text-muted-foreground">
            <li>
              <strong className="text-foreground">SDEX + Soroban in one quote path</strong>{" "}
              — compare venues instead of hopping between apps.
            </li>
            <li>
              <strong className="text-foreground">Non-custodial execution</strong>{" "}
              — Freighter, xBull, Albedo, and LOBSTR sign; StellarRoute never
              holds keys.
            </li>
            <li>
              <strong className="text-foreground">Orderbook visibility</strong>{" "}
              — inspect depth on{" "}
              <Link
                href="/orderbook"
                className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
              >
                the orderbook view
              </Link>{" "}
              when you want more than a single number.
            </li>
            <li>
              <strong className="text-foreground">Cross-chain when needed</strong>{" "}
              — extend beyond Stellar with{" "}
              <Link
                href="/cross-chain-swap"
                className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
              >
                CCTP USDC cross-chain swaps
              </Link>
              .
            </li>
          </ul>
        </section>

        <section className="space-y-6" aria-labelledby="faq-title">
          <h2 id="faq-title" className="font-display text-2xl font-semibold">
            Stellar DEX aggregator FAQ
          </h2>
          <dl className="space-y-6">
            {faqs.map((faq) => (
              <div key={faq.question} className="border-t border-border/70 pt-5">
                <dt className="font-medium text-foreground">{faq.question}</dt>
                <dd className="mt-2 leading-relaxed text-muted-foreground">
                  {faq.answer}
                </dd>
              </div>
            ))}
          </dl>
        </section>
      </div>
    </main>
  );
}
