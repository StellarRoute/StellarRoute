import { fileURLToPath } from "node:url";
import type { NextConfig } from "next";
import bundleAnalyzer from "@next/bundle-analyzer";

const withBundleAnalyzer = bundleAnalyzer({
  enabled: process.env.ANALYZE === "true",
});

const freighterE2EMock = fileURLToPath(
  new URL("./__mocks__/@stellar/freighter-api.browser.ts", import.meta.url)
);

const e2eAliasConfig: Partial<NextConfig> =
  process.env.PLAYWRIGHT_E2E === "true"
    ? {
        // Next 16 defaults to Turbopack; webpack alias is used with `next dev --webpack`.
        turbopack: {},
        webpack: (config, { isServer }) => {
          if (!isServer) {
            config.resolve.alias = {
              ...(config.resolve.alias ?? {}),
              "@stellar/freighter-api": freighterE2EMock,
            };
          }
          return config;
        },
      }
    : {};

const nextConfig: NextConfig = {
  reactStrictMode: true,
  poweredByHeader: false,
  images: {
    remotePatterns: [],
  },
  async rewrites() {
    const backendUrl = process.env.BACKEND_URL;
    if (!backendUrl) return [];
    return [
      {
        source: "/api/v1/:path*",
        destination: `${backendUrl}/api/v1/:path*`,
      },
    ];
  },
  ...e2eAliasConfig,
};

export default withBundleAnalyzer(nextConfig);
