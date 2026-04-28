'use client';

import { HelpCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Skeleton } from "@/components/ui/skeleton";
import { PriceImpactIndicator } from "./PriceImpactIndicator";
import { Button } from "@/components/ui/button";
import { useSwapI18n } from "@/lib/swap-i18n";
import PriceSparkline from "@/components/shared/PriceSparkline";

interface PriceInfoPanelProps {
  rate?: string;
  priceImpact?: number;
  minReceived?: string;
  networkFee?: string;
  isLoading?: boolean;
  onExportJson?: () => void;
  onExportCsv?: () => void;
}

export function PriceInfoPanel({
  rate,
  priceImpact = 0,
  minReceived,
  networkFee,
  isLoading = false,
  onExportJson,
  onExportCsv,
}: PriceInfoPanelProps) {
  const { t } = useSwapI18n();

  // ✅ TEMP MOCK DATA (replace later with real API)
  const mockPriceData = [
    { timestamp: 1710000000000, price: 100 },
    { timestamp: 1710003600000, price: 105 },
    { timestamp: 1710007200000, price: 102 },
    { timestamp: 1710010800000, price: 110 },
    { timestamp: 1710014400000, price: 108 },
    { timestamp: 1710018000000, price: 112 },
    { timestamp: 1710021600000, price: 109 },
  ];

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
    <div className="rounded-2xl border border-border/40 bg-background/40 backdrop-blur-sm p-4 space-y-4 transition-all duration-300 hover:border-primary/20">
      
      {/* 🔥 NEW: Sparkline Section */}
      <div>
        <div className="text-xs text-muted-foreground mb-1">
          24h Price Trend
        </div>
        <PriceSparkline data={mockPriceData} />
      </div>

      {/* Existing UI */}
      <div className="flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>{t("swap.quote.rate")}</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">{t("swap.quote.exchangeRateTooltip")}</p>
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
          <span>{t("swap.quote.priceImpact")}</span>
        </div>
        <PriceImpactIndicator impact={priceImpact} />
      </div>

      <div className="flex justify-between items-center text-sm">
        <div className="flex items-center gap-1.5 text-muted-foreground font-medium">
          <span>{t("swap.quote.minimumReceived")}</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">{t("swap.quote.minimumReceivedTooltip")}</p>
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
          <span>{t("swap.quote.networkFee")}</span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <HelpCircle className="h-3.5 w-3.5 opacity-50 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">{t("swap.quote.networkFeeTooltip")}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <span className="font-medium text-foreground/70 tabular-nums">
          {networkFee || '—'}
        </span>
      </div>

      <div className="pt-2 flex flex-wrap justify-end gap-2">
        <Button size="sm" variant="outline" type="button" onClick={onExportJson}>
          {t("swap.quote.exportJson")}
        </Button>
        <Button size="sm" variant="outline" type="button" onClick={onExportCsv}>
          {t("swap.quote.exportCsv")}
        </Button>
      </div>
    </div>
  );
}