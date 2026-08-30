import type { Metadata } from "next";

/** Live Vercel host redirects apex → www; keep canonicals on www. */
export const DEFAULT_SITE_URL = "https://www.stellarroute.app";

export const SITE_NAME = "StellarRoute";

export const DEFAULT_TITLE =
  "StellarRoute — Stellar DEX Aggregator & Cross-Chain Swap";

export const DEFAULT_DESCRIPTION =
  "Non-custodial Stellar DEX aggregator for SDEX and Soroban AMMs, plus cross-chain USDC swaps via Circle CCTP. Best-price routing without giving up custody.";

export function getSiteUrl(): string {
  const configured = process.env.NEXT_PUBLIC_SITE_URL?.trim();
  if (!configured) return DEFAULT_SITE_URL;
  return configured.replace(/\/$/, "");
}

export function absoluteUrl(path = "/"): string {
  const base = getSiteUrl();
  if (!path || path === "/") return base;
  return `${base}${path.startsWith("/") ? path : `/${path}`}`;
}

type BuildMetadataInput = {
  title: string;
  description: string;
  path: string;
  /** When true, title is used as-is (no "| StellarRoute" template). */
  absoluteTitle?: boolean;
};

export function buildPageMetadata({
  title,
  description,
  path,
  absoluteTitle = false,
}: BuildMetadataInput): Metadata {
  const url = absoluteUrl(path);
  const resolvedTitle = absoluteTitle
    ? title
    : title.includes(SITE_NAME)
      ? title
      : `${title} | ${SITE_NAME}`;

  return {
    title: absoluteTitle ? { absolute: title } : title,
    description,
    alternates: {
      canonical: url,
    },
    openGraph: {
      title: resolvedTitle,
      description,
      url,
      siteName: SITE_NAME,
      type: "website",
      images: [
        {
          url: "/icons/icon-512.svg",
          width: 512,
          height: 512,
          alt: "StellarRoute logo",
        },
      ],
    },
    twitter: {
      card: "summary_large_image",
      title: resolvedTitle,
      description,
      images: ["/icons/icon-512.svg"],
    },
  };
}

export function organizationJsonLd() {
  return {
    "@context": "https://schema.org",
    "@type": "Organization",
    name: SITE_NAME,
    url: getSiteUrl(),
    logo: absoluteUrl("/icons/icon-512.svg"),
    sameAs: ["https://github.com/StellarRoute/StellarRoute"],
    description: DEFAULT_DESCRIPTION,
  };
}

export function websiteJsonLd() {
  return {
    "@context": "https://schema.org",
    "@type": "WebSite",
    name: SITE_NAME,
    url: getSiteUrl(),
    description: DEFAULT_DESCRIPTION,
    publisher: {
      "@type": "Organization",
      name: SITE_NAME,
    },
  };
}

export function softwareApplicationJsonLd() {
  return {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: SITE_NAME,
    applicationCategory: "FinanceApplication",
    operatingSystem: "Web",
    url: getSiteUrl(),
    description: DEFAULT_DESCRIPTION,
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    featureList: [
      "Stellar DEX aggregator",
      "SDEX and Soroban AMM routing",
      "Cross-chain USDC swap via Circle CCTP",
      "Non-custodial wallet execution",
    ],
  };
}

export function faqJsonLd(
  faqs: ReadonlyArray<{ question: string; answer: string }>,
) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: faqs.map((faq) => ({
      "@type": "Question",
      name: faq.question,
      acceptedAnswer: {
        "@type": "Answer",
        text: faq.answer,
      },
    })),
  };
}

export const HOME_FAQS = [
  {
    question: "What is StellarRoute?",
    answer:
      "StellarRoute is a non-custodial Stellar DEX aggregator. It compares executable liquidity across the Stellar SDEX and Soroban AMMs, then lets your wallet sign the swap.",
  },
  {
    question: "Does StellarRoute support cross-chain swaps?",
    answer:
      "Yes. StellarRoute supports cross-chain USDC transfers between Stellar and Ethereum (Sepolia testnet today) using Circle CCTP, in addition to Stellar-native DEX routing.",
  },
  {
    question: "Is StellarRoute custodial?",
    answer:
      "No. StellarRoute builds the route; your connected wallet (Freighter, xBull, Albedo, LOBSTR, or an EVM wallet for destination mint) reviews and signs. StellarRoute never holds your keys.",
  },
  {
    question: "How is StellarRoute different from a single Stellar DEX?",
    answer:
      "A single venue only sees its own book or pool. StellarRoute aggregates Stellar DEX and Soroban AMM liquidity so you can compare routes and aim for better executable price from one interface.",
  },
] as const;
