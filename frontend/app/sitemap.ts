import type { MetadataRoute } from "next";

const DEFAULT_SITE_URL = "https://stellarroute.app";

const PUBLIC_MARKETING_ROUTES = ["/", "/swap", "/orderbook", "/status"] as const;

function getSiteUrl(): string {
  const configured = process.env.NEXT_PUBLIC_SITE_URL?.trim();
  if (!configured) return DEFAULT_SITE_URL;
  return configured.replace(/\/$/, "");
}

export default function sitemap(): MetadataRoute.Sitemap {
  const siteUrl = getSiteUrl();
  const lastModified = new Date();

  return PUBLIC_MARKETING_ROUTES.map((route) => ({
    url: route === "/" ? siteUrl : `${siteUrl}${route}`,
    lastModified,
    changeFrequency: route === "/" ? "weekly" : "daily",
    priority: route === "/" ? 1 : 0.8,
  }));
}
