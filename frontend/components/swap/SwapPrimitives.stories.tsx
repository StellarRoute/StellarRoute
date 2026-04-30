import type { Story } from "@ladle/react";
import { useState } from "react";
import { TokenSelector } from "./TokenSelector";
import { QuoteSummary } from "./QuoteSummary";
import { RouteDisplay, AlternativeRoute } from "./RouteDisplay";
import { SlippageControl } from "./SlippageControl";

// Mock data for RouteDisplay
const mockAlternativeRoutes: AlternativeRoute[] = [
  {
    id: "route-1",
    venue: "AQUA Pool",
    expectedAmount: "≈ 100.5000",
    hops: [
      { id: "h1", fromAsset: "XLM", toAsset: "USDC", venue: "AQUA Pool", fee: "0.00001 XLM" }
    ]
  },
  {
    id: "route-2",
    venue: "SDEX",
    expectedAmount: "≈ 99.8000",
    hops: [
      { id: "h2", fromAsset: "XLM", toAsset: "USDC", venue: "SDEX", fee: "0.00005 XLM" }
    ]
  }
];

export const TokenSelectorStory: Story = () => {
  const [selected, setSelected] = useState("native");
  return (
    <div className="p-8 max-w-md space-y-8">
      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-4">Token Selector - Default</h3>
        <TokenSelector selectedAsset={selected} onSelect={setSelected} />
      </div>
      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-4">Token Selector - Loading</h3>
        <TokenSelector selectedAsset="native" onSelect={() => {}} isLoading />
      </div>
      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-4">Token Selector - Disabled</h3>
        <TokenSelector selectedAsset="native" onSelect={() => {}} disabled />
      </div>
    </div>
  );
};
TokenSelectorStory.storyName = "Token Selector";

export const QuoteSummaryStory: Story = () => (
  <div className="p-8 max-w-md space-y-8">
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Quote Summary - Default</h3>
      <QuoteSummary 
        rate="1 XLM = 0.105 USDC" 
        fee="0.00001 XLM" 
        priceImpact="< 0.01%" 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Quote Summary - Loading</h3>
      <QuoteSummary 
        rate="" 
        fee="" 
        priceImpact="" 
        isLoading 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Quote Summary - Error</h3>
      <QuoteSummary 
        rate="" 
        fee="" 
        priceImpact="" 
        error="Unable to fetch quote from SDEX. Please try again." 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Quote Summary - Empty</h3>
      <QuoteSummary 
        rate="" 
        fee="" 
        priceImpact="" 
      />
    </div>
  </div>
);
QuoteSummaryStory.storyName = "Quote Card (Summary)";

export const RouteDisplayStory: Story = () => (
  <div className="p-8 max-w-xl space-y-8">
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Route Display - Optimal</h3>
      <RouteDisplay 
        amountOut="10.5 USDC" 
        confidenceScore={98} 
        volatility="low" 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Route Display - Alternatives</h3>
      <RouteDisplay 
        amountOut="10.5 USDC" 
        alternativeRoutes={mockAlternativeRoutes}
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Route Display - Loading</h3>
      <RouteDisplay 
        amountOut="" 
        isLoading 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Route Display - Error</h3>
      <RouteDisplay 
        amountOut="" 
        error="No valid route found for this pair." 
      />
    </div>
    <div>
      <h3 className="text-sm font-medium text-muted-foreground mb-4">Route Display - Empty</h3>
      <RouteDisplay 
        amountOut="" 
      />
    </div>
  </div>
);
RouteDisplayStory.storyName = "Route Row (Display)";

export const SlippageControlStory: Story = () => {
  const [slippage, setSlippage] = useState(0.5);
  return (
    <div className="p-8 max-w-md space-y-8">
      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-4">Slippage Control</h3>
        <div className="flex items-center gap-4">
          <span>Current: {slippage}%</span>
          <SlippageControl slippage={slippage} onChange={setSlippage} />
        </div>
      </div>
    </div>
  );
};
SlippageControlStory.storyName = "Slippage Control";
