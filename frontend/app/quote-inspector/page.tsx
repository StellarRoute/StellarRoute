"use client";

import { QuoteInspector, VenueQuote } from "@/components/shared/QuoteInspector";
import { toast } from "sonner";
import { Header } from "@/components/Header";

const MOCK_TIMESTAMP = 1713895200; // Fixed timestamp to satisfy purity rule

const mockQuotes: VenueQuote[] = [
  {
    base_asset: { asset_type: "native" },
    quote_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
    amount: "1000",
    price: "0.1052",
    total: "105.20",
    quote_type: "sell",
    timestamp: MOCK_TIMESTAMP,
    venueName: "Stellar SDEX",
    path: [
      {
        from_asset: { asset_type: "native" },
        to_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
        price: "0.1052",
        source: "sdex"
      }
    ]
  },
  {
    base_asset: { asset_type: "native" },
    quote_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
    amount: "1000",
    price: "0.1061",
    total: "106.10",
    quote_type: "sell",
    timestamp: MOCK_TIMESTAMP,
    venueName: "Soroban AMM (Phoenix)",
    isAggregated: true,
    path: [
      {
        from_asset: { asset_type: "native" },
        to_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
        price: "0.1061",
        source: "amm:phoenix_pool_address"
      }
    ]
  },
  {
    base_asset: { asset_type: "native" },
    quote_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
    amount: "1000",
    price: "0.1045",
    total: "104.50",
    quote_type: "sell",
    timestamp: MOCK_TIMESTAMP,
    venueName: "Stellar-AMM (XLM/USDC)",
    path: [
      {
        from_asset: { asset_type: "native" },
        to_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z..." },
        price: "0.1045",
        source: "amm:stellar_native_pool"
      }
    ]
  }
];

export default function QuoteInspectorPage() {

  const handleSelect = (quote: VenueQuote) => {
    toast.success(`Quote reconciled via ${quote.venueName}`, {
      description: `Final output: ${quote.total} USDC`,
    });
  };

  return (
    <div className="min-h-screen bg-background">
      <Header />
      <main className="container mx-auto px-4 py-12">
        <div className="max-w-4xl mx-auto space-y-8">
          <div className="text-center space-y-2">
            <h1 className="text-4xl font-black tracking-tight text-foreground">
              Quote Reconciliation
            </h1>
            <p className="text-muted-foreground text-lg">
              Analyze and reconcile liquidity paths across multiple Stellar venues.
            </p>
          </div>

          <QuoteInspector 
            quotes={mockQuotes} 
            onSelect={handleSelect} 
          />
        </div>
      </main>
    </div>
  );
}
