'use client';

import { Info, HelpCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import { PriceImpactIndicator } from "./PriceImpactIndicator";

interface PriceInfoPanelProps {
  rate?: string;
  priceImpact?: number;
  minReceived?: string;
  networkFee?: string;
  isLoading?: boolean;
}

export function PriceInfoPanel({
  rate,
  priceImpact = 0,
  minReceived,
  networkFee,
  isLoading = false,
}: PriceInfoPanelProps) {
  if (isLoading) {
    return (
      <div className="rounded-2xl border border-border/40 bg-background/40 backdrop-blur-sm p-4 space-y-3">
        <Skeleton className="h-4 w-full opacity-50" />
        <Skeleton className="h-4 w-3/4 opacity-50" />
        <Skeleton className="h-4 w-1/2 opacity-50" />
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-border/40 bg-background/40 backdrop-blur-sm p-4 space-y-3 transition-all duration-300 hover:border-primary/20">
      <div className="flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>Exchange Rate</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">Current market rate for this trading pair inclusive of path routing.</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <span className="font-bold text-foreground/90 tabular-nums">
          {rate || '—'}
        </span>
      </div>

      <div className="flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>Price Impact</span>
        </div>
        <PriceImpactIndicator impact={priceImpact} />
      </div>

      <div className="flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>Minimum Received</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">Your transaction will revert if there is a large unfavorable price movement before it is confirmed.</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <span className="font-bold text-foreground/90 tabular-nums">
          {minReceived || '—'}
        </span>
      </div>

      <div className="pt-2 mt-1 border-t border-border/20 flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>Network Fee</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">Estimated cost to execute this transaction on the Stellar network.</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <span className="font-medium text-foreground/70 tabular-nums">
          {networkFee || '—'}
        </span>
      </div>
    </div>
  );
}

