import Link from "next/link";

import { HOME_FAQS } from "@/lib/seo";

export function HomeFaqSection() {
  return (
    <section
      aria-labelledby="home-faq-title"
      className="border-t border-border/70 bg-card/20"
    >
      <div className="container mx-auto px-4 py-20 sm:px-6 sm:py-24 lg:px-8">
        <div className="max-w-3xl">
          <p className="font-mono text-xs font-medium uppercase tracking-[0.22em] text-primary">
            FAQ
          </p>
          <h2
            id="home-faq-title"
            className="font-display mt-4 text-3xl font-semibold tracking-[-0.04em] sm:text-4xl"
          >
            Stellar DEX and cross-chain swap questions
          </h2>
          <p className="mt-4 text-muted-foreground leading-relaxed">
            Short answers for traders comparing a Stellar DEX aggregator with
            cross-chain USDC corridors. See also{" "}
            <Link
              href="/stellar-dex-aggregator"
              className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
            >
              how Stellar DEX aggregation works
            </Link>{" "}
            and{" "}
            <Link
              href="/cross-chain-swap"
              className="font-medium text-foreground underline decoration-border underline-offset-4 hover:decoration-primary"
            >
              Stellar cross-chain swaps
            </Link>
            .
          </p>
        </div>

        <dl className="mt-12 max-w-3xl space-y-8">
          {HOME_FAQS.map((faq) => (
            <div key={faq.question} className="border-t border-border/70 pt-6">
              <dt className="font-display text-lg font-semibold text-foreground">
                {faq.question}
              </dt>
              <dd className="mt-2 leading-relaxed text-muted-foreground">
                {faq.answer}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  );
}
