'use client';

import { Info, AlertCircle, AlertTriangle, CheckCircle2 } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface PriceImpactIndicatorProps {
  impact: number;
  className?: string;
  showIcon?: boolean;
}

export function PriceImpactIndicator({
  impact,
  className,
  showIcon = true,
}: PriceImpactIndicatorProps) {
  const getImpactData = (val: number) => {
    if (val < 1) return {
      color: "text-emerald-500",
      label: "Safe",
      icon: <CheckCircle2 className="h-3.5 w-3.5" />,
      description: "Low price impact. Your trade is unlikely to significantly move the market price."
    };
    if (val < 3) return {
      color: "text-yellow-500",
      label: "Moderate",
      icon: <Info className="h-3.5 w-3.5" />,
      description: "Moderate price impact. This trade size will slightly move the market price."
    };
    if (val < 5) return {
      color: "text-orange-500",
      label: "High",
      icon: <AlertTriangle className="h-3.5 w-3.5" />,
      description: "High price impact. This trade will significantly move the market price and lead to less favorable execution."
    };
    return {
      color: "text-destructive",
      label: "Very High",
      icon: <AlertCircle className="h-3.5 w-3.5" />,
      description: "Very high price impact. This trade will severely move the market price. Check liquidity or consider splitting the trade."
    };
  };

  const data = getImpactData(impact);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={cn("flex items-center gap-1.5 cursor-help transition-opacity hover:opacity-80", data.color, className)}>
            <span className="font-bold tabular-nums">
              {impact > 0 ? `${impact.toFixed(2)}%` : '< 0.01%'}
            </span>
            {showIcon && data.icon}
          </div>
        </TooltipTrigger>
        <TooltipContent className="max-w-[240px] p-3 text-xs leading-relaxed">
          <p className="font-bold mb-1">Price Impact ({data.label})</p>
          <p className="text-muted-foreground">
            {data.description}
          </p>
          <div className="mt-2 pt-2 border-t border-border/20 italic">
            Price impact is the difference between the market price and the estimated execution price caused by your trade size.
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
