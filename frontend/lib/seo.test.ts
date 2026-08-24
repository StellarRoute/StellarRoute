import { describe, expect, it } from "vitest";

import {
  DEFAULT_SITE_URL,
  absoluteUrl,
  buildPageMetadata,
  faqJsonLd,
  getSiteUrl,
} from "./seo";

describe("seo helpers", () => {
  it("defaults to the www production host", () => {
    expect(getSiteUrl()).toBe(DEFAULT_SITE_URL);
    expect(absoluteUrl("/swap")).toBe(`${DEFAULT_SITE_URL}/swap`);
    expect(absoluteUrl("/")).toBe(DEFAULT_SITE_URL);
  });

  it("builds canonical + social metadata for a path", () => {
    const meta = buildPageMetadata({
      title: "Cross-Chain Swap",
      description: "Stellar to Ethereum USDC via CCTP.",
      path: "/cross-chain-swap",
    });

    expect(meta.alternates?.canonical).toBe(
      `${DEFAULT_SITE_URL}/cross-chain-swap`,
    );
    expect(meta.openGraph?.url).toBe(`${DEFAULT_SITE_URL}/cross-chain-swap`);
    expect(meta.description).toContain("CCTP");
  });

  it("emits FAQPage JSON-LD entities", () => {
    const ld = faqJsonLd([
      { question: "What is a Stellar DEX aggregator?", answer: "Routes across venues." },
    ]);
    expect(ld["@type"]).toBe("FAQPage");
    expect(ld.mainEntity).toHaveLength(1);
  });
});
