"use client";

import { Info, DollarSign, Layers, TrendingDown } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface FeeComponent {
  name: string;
  amount: string;
  description: string;
}

interface FeeBreakdownPanelProps {
  /** Protocol fees breakdown */
  protocolFees: FeeComponent[];
  /** Estimated network costs */
  networkCosts: FeeComponent[];
  /** Net output after fees */
  netOutput: string;
  /** Total fee estimate */
  totalFee: string;
  /** Whether data is available */
  isDataAvailable?: boolean;
}

/**
 * Fee breakdown panel with detailed cost analysis
 * Shows per-fee component with hover tooltips
 * Gracefully handles unavailable estimate data
 */
export function FeeBreakdownPanel({
  protocolFees,
  networkCosts,
  netOutput,
  totalFee,
  isDataAvailable = true,
}: FeeBreakdownPanelProps) {
  if (!isDataAvailable) {
    return (
      <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
        <div className="flex items-center gap-2 text-muted-foreground">
          <DollarSign className="h-4 w-4" />
          <span className="text-sm font-medium">Fee Estimate</span>
        </div>
        <p className="text-xs text-muted-foreground text-center py-4">
          Fee estimates are currently unavailable. Please try again later.
        </p>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-4 bg-muted/30">
      <div className="flex items-center gap-2">
        <DollarSign className="h-4 w-4 text-primary" />
        <span className="text-sm font-medium">Fee Breakdown</span>
      </div>

      {/* Protocol Fees Section */}
      <div className="space-y-2">
        <div className="flex items-center gap-1.5">
          <Layers className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Protocol Fees
          </span>
        </div>
        <div className="space-y-1.5 pl-1">
          {protocolFees.map((fee, index) => (
            <TooltipProvider key={index}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div className="flex justify-between items-center text-sm cursor-help">
                    <span className="text-muted-foreground flex items-center gap-1">
                      {fee.name}
                      <Info className="h-3 w-3 text-muted-foreground/50" />
                    </span>
                    <span className="font-medium">{fee.amount}</span>
                  </div>
                </TooltipTrigger>
                <TooltipContent side="left" className="max-w-[200px]">
                  <p className="text-xs">{fee.description}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ))}
        </div>
      </div>

      {/* Network Costs Section */}
      <div className="space-y-2">
        <div className="flex items-center gap-1.5">
          <TrendingDown className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Network Costs
          </span>
        </div>
        <div className="space-y-1.5 pl-1">
          {networkCosts.map((cost, index) => (
            <TooltipProvider key={index}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div className="flex justify-between items-center text-sm cursor-help">
                    <span className="text-muted-foreground flex items-center gap-1">
                      {cost.name}
                      <Info className="h-3 w-3 text-muted-foreground/50" />
                    </span>
                    <span className="font-medium">{cost.amount}</span>
                  </div>
                </TooltipTrigger>
                <TooltipContent side="left" className="max-w-[200px]">
                  <p className="text-xs">{cost.description}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ))}
        </div>
      </div>

      {/* Total and Net Output */}
      <div className="pt-3 border-t border-border/50 space-y-2">
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground font-medium">Total Fees</span>
          <span className="font-semibold text-amber-600">{totalFee}</span>
        </div>
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground font-medium">Net Output</span>
          <span className="font-semibold text-emerald-600">{netOutput}</span>
        </div>
      </div>
    </div>
  );
}