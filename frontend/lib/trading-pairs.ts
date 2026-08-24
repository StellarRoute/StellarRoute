import type { TradingPair } from "@/types";

/** Demo issuer used by the staged testnet orderbook fixtures. */
export const STAGING_DEMO_ISSUER =
  "GDMVY5CPSEY6IDQBEX7KMJSOVFNHMOMT5QY4MTOCSDFORV24AOFYDDGS";

export function assetCode(asset: string): string {
  if (asset === "native") return "XLM";
  return asset.split(":")[0] || asset;
}

/** True when either offer direction is indexed for this asset pair. */
export function pairExists(
  a: string,
  b: string,
  pairs: TradingPair[] | undefined | null
): boolean {
  if (!pairs?.length || !a || !b || a === b) return false;
  return pairs.some(
    (pair) =>
      (pair.base_asset === a && pair.counter_asset === b) ||
      (pair.base_asset === b && pair.counter_asset === a)
  );
}

/** Counter-assets that share an indexed market with `asset`. */
export function counterpartsFor(
  asset: string,
  pairs: TradingPair[] | undefined | null
): string[] {
  if (!pairs?.length || !asset) return [];
  const out = new Set<string>();
  for (const pair of pairs) {
    if (pair.base_asset === asset) out.add(pair.counter_asset);
    if (pair.counter_asset === asset) out.add(pair.base_asset);
  }
  return Array.from(out);
}

const PREFERRED_DEMO_PAIRS: Array<[string, string]> = [
  [`EUR:${STAGING_DEMO_ISSUER}`, `USDy:${STAGING_DEMO_ISSUER}`],
  [`BTC:${STAGING_DEMO_ISSUER}`, `EXT:${STAGING_DEMO_ISSUER}`],
  [`BTC:${STAGING_DEMO_ISSUER}`, `USDy:${STAGING_DEMO_ISSUER}`],
  [`EXT:${STAGING_DEMO_ISSUER}`, `USDy:${STAGING_DEMO_ISSUER}`],
  [`EURy:${STAGING_DEMO_ISSUER}`, `USD:${STAGING_DEMO_ISSUER}`],
  [`ARS:${STAGING_DEMO_ISSUER}`, `EURy:${STAGING_DEMO_ISSUER}`],
  [`ARS:${STAGING_DEMO_ISSUER}`, `USDy:${STAGING_DEMO_ISSUER}`],
];

/**
 * Pick a demo pair that is actually indexed. Prefer non-native GDMVY5 markets
 * because native (XLM) quote resolution is still flaky on staging.
 */
export function pickPreferredDemoPair(
  pairs: TradingPair[] | undefined | null
): { from: string; to: string } | null {
  if (!pairs?.length) return null;

  for (const [from, to] of PREFERRED_DEMO_PAIRS) {
    if (pairExists(from, to, pairs)) return { from, to };
  }

  const nonNative = pairs.find(
    (pair) => pair.base_asset !== "native" && pair.counter_asset !== "native"
  );
  if (nonNative) {
    return { from: nonNative.base_asset, to: nonNative.counter_asset };
  }

  const any = pairs[0];
  return { from: any.base_asset, to: any.counter_asset };
}
