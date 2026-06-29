import type { MetadataRoute } from "next";

const DEFAULT_SITE_URL = "https://stellarroute.app";

function getSiteUrl(): string {
  const configured = process.env.NEXT_PUBLIC_SITE_URL?.trim();
  if (!configured) return DEFAULT_SITE_URL;
  return configured.replace(/\/$/, "");
}

export default function robots(): MetadataRoute.Robots {
  const siteUrl = getSiteUrl();

  return {
    rules: {
      userAgent: "*",
      allow: "/",
      disallow: ["/demo/"],
    },
    sitemap: `${siteUrl}/sitemap.xml`,
  };
}
