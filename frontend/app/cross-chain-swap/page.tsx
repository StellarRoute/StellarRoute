import Link from "next/link";
import { ArrowRight } from "lucide-react";

import { JsonLd } from "@/components/seo/JsonLd";
import { Button } from "@/components/ui/button";
import { buildPageMetadata, faqJsonLd } from "@/lib/seo";

const faqs = [
  {
    question: "What is a Stellar cross-chain swap?",
    answer:
      "A Stellar cross-chain swap moves value between Stellar and another network. On StellarRoute, the live corridor uses Circle CCTP to burn and mint native USDC between Stellar Testnet and Ethereum Sepolia.",
  },
  {
    question: "Which chains does StellarRoute support today?",
    answer:
      "Stellar-native swaps run on the Stellar SDEX and Soroban AMMs. Cross-chain USDC currently targets the Stellar ↔ Ethereum Sepolia CCTP corridor, with public mainnet enablement operator-gated.",
  },
  {
    question: "Do I give up custody during a cross-chain swap?",
    answer:
      "No. You authorize each step from your wallets. StellarRoute prepares the route; Freighter (or another Stellar wallet) and an EVM wallet sign their respective steps.",
  },
] as const;

export const metadata = buildPageMetadata({
  title: "Cross-Chain Swap on Stellar",
  description:
    "Non-custodial cross-chain USDC swaps between Stellar and Ethereum via Circle CCTP, plus Stellar DEX routing in one StellarRoute deck.",
  path: "/cross-chain-swap",
});

export default function CrossChainSwapPage() {
  return (
    <main className="min-h-[calc(100vh-80px)] px-4 py-12 sm:px-6 lg:px-8">
      <JsonLd data={faqJsonLd(faqs)} />
      <div className="container mx-auto max-w-3xl space-y-12">
        <header className="space-y-4">
          <p className="font-mono text-xs font-medium uppercase tracking-[0.22em] text-primary">
            Cross-chain swap
          </p>
          <h1 className="font-display text-4xl font-semibold tracking-[-0.045em] sm:text-5xl">
            Cross-chain swap for Stellar USDC
          </h1>
          <p className="max-w-2xl text-lg leading-relaxed text-muted-foreground">
            Move native USDC between Stellar and Ethereum with Circle CCTP —
            without custodial bridges. Pair it with Stellar DEX aggregation when
            you need a best-price on-chain leg first.
          </p>
          <div className="flex flex-wrap gap-3 pt-2">
            <Button asChild size="lg" className="h-11">
              <Link href="/swap">
                Open cross-chain deck
                <ArrowRight className="h-4 w-4" aria-hidden="true" />
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg" className="h-11">
              <Link href="/stellar-dex-aggregator">Stellar DEX aggregator</Link>
            </Button>
          </div>
        </header>

        <section className="space-y-4" aria-labelledby="how-title">
          <h2 id="how-title" className="font-display text-2xl font-semibold">
            How a StellarRoute cross-chain swap works
          </h2>
          <ol className="space-y-4 text-muted-foreground leading-relaxed">
            <li>
              <strong className="text-foreground">1. Choose the corridor.</strong>{" "}
              Pick Stellar → Ethereum or Ethereum → Stellar for native USDC via
              CCTP.
            </li>
            <li>
              <strong className="text-foreground">2. Quote and review.</strong>{" "}
              StellarRoute shows the burn/mint path, fees, and attestation
              wait — including Fast vs Standard finality when available.
            </li>
            <li>
              <strong className="text-foreground">3. Sign in your wallets.</strong>{" "}
              Authorize the Stellar burn (or EVM burn) and destination mint.
              Keys stay in your wallets.
            </li>
          </ol>
        </section>

        <section className="space-y-4" aria-labelledby="why-title">
          <h2 id="why-title" className="font-display text-2xl font-semibold">
            Why traders look for Stellar cross-chain swaps here
          </h2>
          <ul className="space-y-3 text-muted-foreground leading-relaxed">
            <li>
              <strong className="text-foreground">Native USDC, not wrapped IOUs</strong>{" "}
              — Circle CCTP burns and mints the canonical USDC on each side.
            </li>
            <li>
              <strong className="text-foreground">Same product as Stellar DEX routing</strong>{" "}
              — aggregate SDEX + Soroban, then bridge when the destination is
              another chain.
            </li>
            <li>
              <strong className="text-foreground">Signed-live testnet proof</strong>{" "}
              — both directions have public mint evidence on Sepolia and Stellar
              Testnet.
            </li>
          </ul>
        </section>

        <section className="space-y-6" aria-labelledby="faq-title">
          <h2 id="faq-title" className="font-display text-2xl font-semibold">
            Cross-chain swap FAQ
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

        <p className="text-sm text-muted-foreground">
          New to Stellar swaps? Read the{" "}
          <Link
            href="/guide"
            className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
          >
            first live swap guide
          </Link>
          .
        </p>
      </div>
    </main>
  );
}
