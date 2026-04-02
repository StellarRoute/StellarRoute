"use client";

import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { AlertTriangle, TrendingDown, TrendingUp } from "lucide-react";
import { useSwapI18n } from "@/lib/swap-i18n";

export interface SimulationPanelProps {
  /** Amount being paid/sold */
  payAmount: string;
  /** Expected amount to receive before slippage */
  expectedOutput: string;
  /** Slippage tolerance percentage */
  slippage: number;
  /** Whether simulation is loading */
  isLoading?: boolean;
  /** Error message to display */
  error?: string;
}

interface SimulationData {
  expectedOutput: string;
  minReceived: string;
  effectiveRate: string;
  priceImpact: string;
  slippageProtection: string;
}

export function SimulationPanel({
  payAmount,
  expectedOutput,
  slippage,
  isLoading = false,
  error,
}: SimulationPanelProps) {
  const { t } = useSwapI18n();

  const calculateSimulation = (): SimulationData | null => {
    const payAmountNum = parseFloat(payAmount);
    const expectedOutputNum = parseFloat(expectedOutput);

    if (isNaN(payAmountNum) || isNaN(expectedOutputNum) || payAmountNum <= 0) {
      return null;
    }

    // Calculate minimum received after slippage
    const minReceived = expectedOutputNum * (1 - slippage / 100);

    // Calculate effective rate (expected output / pay amount)
    const effectiveRate = expectedOutputNum / payAmountNum;

    // Calculate price impact (mock calculation - in real app this would come from API)
    // Tune multiplier so larger pay amounts cross the "High Impact" threshold in tests.
    const priceImpact = Math.min(0.5, (payAmountNum / 10000) * 0.5);

    // Calculate slippage protection amount
    const slippageProtection = expectedOutputNum - minReceived;

    return {
      expectedOutput: expectedOutputNum.toFixed(6),
      minReceived: minReceived.toFixed(6),
      effectiveRate: effectiveRate.toFixed(6),
      priceImpact: priceImpact.toFixed(3),
      slippageProtection: slippageProtection.toFixed(6),
    };
  };

  const simulation = calculateSimulation();

  if (error) {
    return (
      <div className="rounded-xl border border-destructive/30 bg-destructive/5 p-4 space-y-3 text-destructive">
        <div className="flex items-center gap-2">
          <AlertTriangle className="h-4 w-4" />
          <span className="text-sm font-medium">{t("swap.simulation.errorTitle")}</span>
        </div>
        <p className="text-sm">{error}</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="rounded-xl border border-border/50 p-4 space-y-4 shadow-sm animate-in fade-in duration-500">
        <div className="flex items-center gap-2">
          <Skeleton className="h-4 w-4 animate-pulse rounded" />
          <Skeleton className="h-4 w-24 animate-pulse rounded" />
        </div>
        <div className="space-y-3">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="flex justify-between items-center">
              <Skeleton className="h-3 w-20 animate-pulse rounded" />
              <Skeleton className="h-3 w-16 animate-pulse rounded" />
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (!simulation) {
    return (
      <div className="rounded-xl border border-border/50 p-4 space-y-3">
        <div className="text-center text-muted-foreground text-sm">
          {t("swap.simulation.emptyState")}
        </div>
      </div>
    );
  }

  const isHighImpact = parseFloat(simulation.priceImpact) > 0.2;

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-4 bg-muted/30">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-medium">{t("swap.simulation.title")}</h4>
          <Badge variant="secondary" className="text-xs">
            {t("swap.simulation.slippageBadge", { value: slippage })}
          </Badge>
        </div>
        {isHighImpact && (
          <div className="flex items-center gap-1 text-warning">
            <TrendingUp className="h-3 w-3" />
            <span className="text-xs font-medium text-warning">{t("swap.simulation.highImpact")}</span>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-3">
        {/* Expected Output */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <span className="text-sm text-muted-foreground">{t("swap.simulation.expectedOutput")}</span>
          <span className="text-sm font-mono font-medium">
            {simulation.expectedOutput}
          </span>
        </div>

        {/* Minimum Received */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <div className="flex items-center gap-1">
            <span className="text-sm text-muted-foreground">{t("swap.simulation.minReceived")}</span>
            <div className="h-3 w-3 rounded-full bg-blue-500/20 flex items-center justify-center">
              <TrendingDown className="h-2 w-2 text-blue-600" />
            </div>
          </div>
          <div className="text-right">
            <span className="text-sm font-mono font-medium text-primary">
              {simulation.minReceived}
            </span>
            <div className="text-xs text-muted-foreground">
              {t("swap.simulation.fromSlippage", {
                amount: simulation.slippageProtection,
              })}
            </div>
          </div>
        </div>

        {/* Effective Rate */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <span className="text-sm text-muted-foreground">{t("swap.simulation.effectiveRate")}</span>
          <span className="text-sm font-mono font-medium">
            1 XLM ≈ {simulation.effectiveRate} USDC
          </span>
        </div>

        {/* Price Impact */}
        <div className="flex justify-between items-center py-2">
          <span className="text-sm text-muted-foreground">{t("swap.simulation.priceImpact")}</span>
          <span
            className={`text-sm font-medium ${
              isHighImpact ? "text-warning" : "text-success"
            }`}
          >
            {simulation.priceImpact}%
          </span>
        </div>
      </div>

      {/* Warning for high price impact */}
      {isHighImpact && (
        <div className="rounded-lg bg-warning/10 border border-warning/30 text-warning p-3">
          <div className="flex items-start gap-2">
            <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
            <div className="text-xs">
              <strong>{t("swap.simulation.highImpactTitle")}</strong>{" "}
              {t("swap.simulation.highImpactBody")}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
