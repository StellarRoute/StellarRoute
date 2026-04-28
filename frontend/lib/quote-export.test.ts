import { describe, expect, it } from "vitest";

import type { QuoteExportPayload } from "./quote-export";
import { quoteExportToCsv } from "./quote-export";

const payload: QuoteExportPayload = {
  exportedAt: "2026-04-26T00:00:00.000Z",
  market: {
    fromAsset: "XLM",
    toAsset: "USDC",
    fromAmount: "100",
    expectedToAmount: "99.1234",
  },
  pricing: {
    rate: "1 XLM = 0.9912 USDC",
    priceImpactPct: "0.20",
    minimumReceived: "98.9000 USDC",
    networkFee: "0.00001 XLM",
  },
  route: {
    selectedVenue: "SDEX",
    routeSummary: "XLM -> USDC",
  },
};

describe("quote export payload", () => {
  it("serializes a stable JSON payload shape without wallet fields", () => {
    const json = JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
    expect(json).toMatchObject({
      market: {
        fromAsset: "XLM",
        toAsset: "USDC",
      },
      pricing: {
        rate: "1 XLM = 0.9912 USDC",
      },
    });
    expect(JSON.stringify(json)).not.toContain("wallet");
    expect(JSON.stringify(json)).not.toContain("address");
  });

  it("serializes CSV export", () => {
    const csv = quoteExportToCsv(payload);
    expect(csv).toContain("field,value");
    expect(csv).toContain("market_from_asset,XLM");
    expect(csv).toContain("route_selected_venue,SDEX");
  });
});

