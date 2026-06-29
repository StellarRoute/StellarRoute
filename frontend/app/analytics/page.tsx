import type { Metadata } from "next";

import { AnalyticsPageClient } from "./AnalyticsPageClient";

/**
 * /analytics — platform metrics dashboard.
 *
 * Metadata: public, indexable overview of non-sensitive API metrics.
 * Access policy: no authentication required; data is read-only and sourced
 * from /metrics/cache and /metrics/pool. Disable via NEXT_PUBLIC_FEATURE_ANALYTICS.
 */
export const metadata: Metadata = {
  title: "Analytics | StellarRoute",
  description:
    "Platform cache and database metrics for StellarRoute trading infrastructure",
  openGraph: {
    title: "Analytics | StellarRoute",
    description:
      "Monitor quote cache performance and database pool utilisation",
    type: "website",
  },
  robots: {
    index: true,
    follow: true,
  },
};

export default function AnalyticsPage() {
  return <AnalyticsPageClient />;
}
