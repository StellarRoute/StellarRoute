import type { Story } from "@ladle/react";
import { useState } from "react";
import { SplitView } from "./SplitView";
import { RouteDisplay } from "./RouteDisplay";

// ---------------------------------------------------------------------------
// Placeholder panels for story isolation
// ---------------------------------------------------------------------------

function QuotePanel() {
  return (
    <div className="rounded-xl border border-border/50 bg-card p-6 space-y-4">
      <h3 className="text-sm font-semibold">Quote Panel</h3>
      <div className="h-32 rounded-lg bg-muted/40 flex items-center justify-center text-xs text-muted-foreground">
        Swap form
      </div>
    </div>
  );
}

function RoutePanel() {
  return (
    <div className="rounded-xl border border-border/50 bg-card p-4">
      <h3 className="text-sm font-semibold mb-3">Route Details</h3>
      <RouteDisplay amountOut="10.5 USDC" confidenceScore={85} volatility="low" />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stories
// ---------------------------------------------------------------------------

/** Standard (single-column) layout */
export const StandardMode: Story = () => {
  const [isSplit, setIsSplit] = useState(false);
  return (
    <div className="p-8 bg-background min-h-screen">
      <SplitView
        isSplit={isSplit}
        onToggle={() => setIsSplit((v) => !v)}
        primary={<QuotePanel />}
        secondary={<RoutePanel />}
        className="max-w-[960px] mx-auto"
      />
    </div>
  );
};
StandardMode.storyName = "SplitView — Standard Mode";

/** Split (side-by-side) layout */
export const SplitMode: Story = () => {
  const [isSplit, setIsSplit] = useState(true);
  return (
    <div className="p-8 bg-background min-h-screen">
      <SplitView
        isSplit={isSplit}
        onToggle={() => setIsSplit((v) => !v)}
        primary={<QuotePanel />}
        secondary={<RoutePanel />}
        className="max-w-[960px] mx-auto"
      />
    </div>
  );
};
SplitMode.storyName = "SplitView — Split Mode";

/** Interactive toggle between both modes */
export const Interactive: Story = () => {
  const [isSplit, setIsSplit] = useState(false);
  return (
    <div className="p-8 bg-background min-h-screen">
      <p className="text-xs text-muted-foreground mb-4">
        Click the toggle button to switch between standard and split-view layouts.
      </p>
      <SplitView
        isSplit={isSplit}
        onToggle={() => setIsSplit((v) => !v)}
        primary={<QuotePanel />}
        secondary={<RoutePanel />}
        className="max-w-[960px] mx-auto"
      />
    </div>
  );
};
Interactive.storyName = "SplitView — Interactive Toggle";
