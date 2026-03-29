import { Skeleton } from "@/components/ui/skeleton";
import { ArrowRight, ArrowDown, Info, ChevronDown } from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { RouteDisplaySkeleton } from "./RouteDisplaySkeleton";

interface RouteDisplayProps {
  amountOut: string;
  /** Route confidence score (0-100) */
  confidenceScore?: number;
  /** Market volatility level */
  volatility?: "high" | "medium" | "low";
  /** Show loading skeleton */
  isLoading?: boolean;
}

export function RouteDisplay({
  amountOut,
  confidenceScore = 85,
  volatility = "low",
  isLoading = false,
}: RouteDisplayProps) {
  const [showDetails, setShowDetails] = useState(false);

  if (isLoading) {
    return <RouteDisplaySkeleton />;
  }

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-4 transition-all duration-200 hover:border-border hover:shadow-sm focus-within:ring-2 focus-within:ring-primary/20">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-medium">Best Route</h4>
          <Info className="h-4 w-4 text-muted-foreground cursor-help" />
        </div>
        <div className="flex items-center gap-2">
          <ConfidenceIndicator score={confidenceScore} volatility={volatility} />
          <Badge variant="secondary" className="text-xs bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 border-emerald-500/20 transition-colors">
            Optimal
          </Badge>
          {/* Task 5.3: "Show route details" toggle as <button> with 44×44px touch target */}
          <button
            type="button"
            onClick={() => setShowDetails((prev) => !prev)}
            aria-expanded={showDetails}
            aria-label="Show route details"
            className="min-h-[44px] min-w-[44px] flex items-center justify-center rounded-md hover:bg-muted/50 focus:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all duration-150 active:scale-95"
          >
            <ChevronDown
              className={`h-4 w-4 text-muted-foreground transition-transform duration-200 ${showDetails ? "rotate-180" : ""}`}
            />
          </button>
        </div>
      </div>

      {/* Task 5.1: Route path — flex-col on mobile, flex-row on sm+ */}
      <div className="flex flex-col sm:flex-row items-center bg-muted/50 rounded-lg p-3 overflow-hidden gap-1 sm:gap-0 sm:justify-between transition-colors duration-150 hover:bg-muted/70">
        <div className="flex flex-col flex-shrink-0 min-w-[40px] items-center sm:items-start">
          <span className="text-xs font-semibold">XLM</span>
          <span className="text-[10px] text-muted-foreground leading-none">Stellar</span>
        </div>

        <ArrowDown className="h-4 w-4 text-muted-foreground flex-shrink-0 sm:hidden" />
        <ArrowRight className="h-4 w-4 text-muted-foreground mx-auto flex-shrink-0 hidden sm:block" />

        <div className="px-2 py-1 bg-background rounded-md border text-xs font-medium shadow-sm flex-shrink-0 text-center mx-1">
          AQUA Pool
        </div>

        <ArrowDown className="h-4 w-4 text-muted-foreground flex-shrink-0 sm:hidden" />
        <ArrowRight className="h-4 w-4 text-muted-foreground mx-auto flex-shrink-0 hidden sm:block" />

        <div className="flex flex-col text-right flex-shrink-0 min-w-[60px] items-center sm:items-end">
          <span className="text-xs font-semibold">USDC</span>
          <span className="text-[10px] text-muted-foreground truncate max-w-[80px]" title={`${amountOut} expected`}>{amountOut} exp.</span>
        </div>
      </div>

      <div className="pt-3 border-t border-border/50 overflow-x-hidden">
        <h4 className="text-[11px] font-medium text-muted-foreground mb-2 uppercase tracking-wider">Alternative Routes</h4>
        <button
          type="button"
          className="w-full flex flex-wrap items-center justify-between opacity-60 hover:opacity-100 focus:opacity-100 transition-all duration-150 p-1 -mx-1 rounded hover:bg-muted/50 focus:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20 gap-1 text-left active:scale-[0.99]"
          onClick={() => console.log("Selecting alternative route...")}
        >
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className="font-medium">XLM</span>
            <ArrowRight className="h-3 w-3" />
            <span className="border border-border/50 rounded bg-background px-1.5 py-0.5 text-[10px]">SDEX</span>
            <ArrowRight className="h-3 w-3" />
            <span className="font-medium">USDC</span>
          </div>
          <span className="text-xs font-medium text-muted-foreground">≈ {(parseFloat(amountOut) * 0.995).toFixed(4)}</span>
        </button>
      </div>
    </div>
  );
}
