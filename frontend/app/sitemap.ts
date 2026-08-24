import type { MetadataRoute } from "next";

import { getSiteUrl } from "@/lib/seo";

const PUBLIC_MARKETING_ROUTES = [
  { path: "/", changeFrequency: "weekly" as const, priority: 1 },
  { path: "/swap", changeFrequency: "daily" as const, priority: 0.95 },
  {
    path: "/cross-chain-swap",
    changeFrequency: "weekly" as const,
    priority: 0.95,
  },
  {
    path: "/stellar-dex-aggregator",
    changeFrequency: "weekly" as const,
    priority: 0.95,
  },
  { path: "/offramp", changeFrequency: "daily" as const, priority: 0.8 },
  { path: "/orderbook", changeFrequency: "daily" as const, priority: 0.7 },
  { path: "/guide", changeFrequency: "monthly" as const, priority: 0.7 },
  { path: "/docs", changeFrequency: "monthly" as const, priority: 0.6 },
  { path: "/status", changeFrequency: "daily" as const, priority: 0.5 },
] as const;

export default function sitemap(): MetadataRoute.Sitemap {
  const siteUrl = getSiteUrl();
  const lastModified = new Date();

  return PUBLIC_MARKETING_ROUTES.map((route) => ({
    url: route.path === "/" ? siteUrl : `${siteUrl}${route.path}`,
    lastModified,
    changeFrequency: route.changeFrequency,
    priority: route.priority,
  }));
}
