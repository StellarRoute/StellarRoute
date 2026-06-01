'use client';

import { HelpCircle, Info } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface SpreadIndicatorProps {
  midpoint?: string;
  spreadBps?: number;
  isLoading?: boolean;
  className?: string;
}

/**
 * Displays the market spread vs midpoint for the current trading pair.
 * Helps contextualize the quoted price against global market conditions.
 */
export function SpreadIndicator({
  midpoint,
  spreadBps,
  isLoading = false,
  className,
}: SpreadIndicatorProps) {
  if (isLoading) {
    return (
      <div 
        className={cn("flex justify-between items-center text-xs animate-pulse", className)}
        data-testid="spread-indicator-loading"
      >
        <div className="h-3 w-20 bg-muted rounded" />
        <div className="h-3 w-12 bg-muted rounded" />
      </div>
    );
  }

  if (midpoint === undefined || spreadBps === undefined) {
    return null;
  }

  const spreadPercent = (spreadBps / 100).toFixed(2);
  
  // Color code the spread
  // < 0.1% = excellent (green)
  // < 0.5% = good (blue)
  // < 2.0% = wide (amber)
  // > 2.0% = critical (red)
  const spreadColor = 
    spreadBps < 10 ? "text-emerald-500" :
    spreadBps < 50 ? "text-blue-500" :
    spreadBps < 200 ? "text-amber-500" :
    "text-destructive";

  return (
    <div className={cn("flex justify-between items-center text-xs", className)}>
      <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
        <span>Market Spread</span>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <HelpCircle className="h-3 w-3 opacity-50 cursor-help" />
            </TooltipTrigger>
            <TooltipContent className="max-w-[240px] p-3">
              <div className="space-y-2">
                <p className="font-semibold flex items-center gap-1.5">
                  <Info className="h-3.5 w-3.5 text-primary" />
                  What is Market Spread?
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  The difference between the best available buy and sell prices. 
                  A lower spread indicates a more liquid and efficient market.
                </p>
                <div className="pt-1 border-t border-border/40">
                  <p className="text-[10px] text-muted-foreground italic">
                    Midpoint: {parseFloat(midpoint).toFixed(6)}
                  </p>
                </div>
              </div>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
      
      <div className="flex items-center gap-2">
        <span className={cn("font-bold tabular-nums", spreadColor)}>
          {spreadPercent}%
        </span>
      </div>
    </div>
  );
}
