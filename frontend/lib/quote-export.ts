export interface QuoteExportPayload {
  exportedAt: string;
  market: {
    fromAsset: string;
    toAsset: string;
    fromAmount: string;
    expectedToAmount: string;
  };
  pricing: {
    rate: string;
    priceImpactPct: string;
    minimumReceived: string;
    networkFee: string;
  };
  route: {
    selectedVenue: string;
    routeSummary: string;
  };
}

export function quoteExportToCsv(payload: QuoteExportPayload) {
  const rows: Array<[string, string]> = [
    ["exported_at", payload.exportedAt],
    ["market_from_asset", payload.market.fromAsset],
    ["market_to_asset", payload.market.toAsset],
    ["market_from_amount", payload.market.fromAmount],
    ["market_expected_to_amount", payload.market.expectedToAmount],
    ["pricing_rate", payload.pricing.rate],
    ["pricing_price_impact_pct", payload.pricing.priceImpactPct],
    ["pricing_minimum_received", payload.pricing.minimumReceived],
    ["pricing_network_fee", payload.pricing.networkFee],
    ["route_selected_venue", payload.route.selectedVenue],
    ["route_summary", payload.route.routeSummary],
  ];

  return [
    "field,value",
    ...rows.map(([field, value]) => `${escapeCsv(field)},${escapeCsv(value)}`),
  ].join("\n");
}

function escapeCsv(value: string) {
  if (value.includes(",") || value.includes("\"") || value.includes("\n")) {
    return `"${value.replaceAll("\"", "\"\"")}"`;
  }
  return value;
}

