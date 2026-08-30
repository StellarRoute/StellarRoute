import { describe, expect, it } from "vitest";
import type { TradingPair } from "@/types";
import {
  counterpartsFor,
  pairExists,
  pickPreferredDemoPair,
  STAGING_DEMO_ISSUER,
} from "@/lib/trading-pairs";

const I = STAGING_DEMO_ISSUER;

function pair(base: string, counter: string): TradingPair {
  return {
    base: base === "native" ? "XLM" : base.split(":")[0],
    counter: counter === "native" ? "XLM" : counter.split(":")[0],
    base_asset: base,
    counter_asset: counter,
    offer_count: 3,
    last_updated: undefined,
  };
}

describe("trading-pairs helpers", () => {
  const pairs = [
    pair(`EUR:${I}`, `USDy:${I}`),
    pair(`BTC:${I}`, `EXT:${I}`),
    pair("native", `USDy:${I}`),
  ];

  it("detects indexed markets in either direction", () => {
    expect(pairExists(`EUR:${I}`, `USDy:${I}`, pairs)).toBe(true);
    expect(pairExists(`USDy:${I}`, `EUR:${I}`, pairs)).toBe(true);
    expect(pairExists("native", `BTC:${I}`, pairs)).toBe(false);
  });

  it("lists counterparts for an asset", () => {
    expect(counterpartsFor(`USDy:${I}`, pairs).sort()).toEqual(
      [`EUR:${I}`, "native"].sort()
    );
  });

  it("prefers non-native demo pairs over native", () => {
    expect(pickPreferredDemoPair(pairs)).toEqual({
      from: `EUR:${I}`,
      to: `USDy:${I}`,
    });
  });
});
