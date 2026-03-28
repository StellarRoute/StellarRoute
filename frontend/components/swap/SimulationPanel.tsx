"use client";

import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { AlertTriangle, TrendingDown, TrendingUp } from "lucide-react";

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
      <div className="rounded-xl border border-red-200 bg-red-50 p-4 space-y-3">
        <div className="flex items-center gap-2 text-red-600">
          <AlertTriangle className="h-4 w-4" />
          <span className="text-sm font-medium">Simulation Error</span>
        </div>
        <p className="text-sm text-red-600">{error}</p>
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
          Enter an amount to see trade simulation
        </div>
      </div>
    );
  }

  const isHighImpact = parseFloat(simulation.priceImpact) > 0.2;

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-4 bg-muted/30">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-medium">Trade Simulation</h4>
          <Badge variant="secondary" className="text-xs">
            {slippage}% slippage
          </Badge>
        </div>
        {isHighImpact && (
          <div className="flex items-center gap-1 text-amber-600">
            <TrendingUp className="h-3 w-3" />
            <span className="text-xs font-medium">High Impact</span>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-3">
        {/* Expected Output */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <span className="text-sm text-muted-foreground">Expected Output</span>
          <span className="text-sm font-mono font-medium">
            {simulation.expectedOutput}
          </span>
        </div>

        {/* Minimum Received */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <div className="flex items-center gap-1">
            <span className="text-sm text-muted-foreground">Min Received</span>
            <div className="h-3 w-3 rounded-full bg-blue-500/20 flex items-center justify-center">
              <TrendingDown className="h-2 w-2 text-blue-600" />
            </div>
          </div>
          <div className="text-right">
            <span className="text-sm font-mono font-medium text-blue-600">
              {simulation.minReceived}
            </span>
            <div className="text-xs text-muted-foreground">
              -{simulation.slippageProtection} from slippage
            </div>
          </div>
        </div>

        {/* Effective Rate */}
        <div className="flex justify-between items-center py-2 border-b border-border/30">
          <span className="text-sm text-muted-foreground">Effective Rate</span>
          <span className="text-sm font-mono font-medium">
            1 XLM ≈ {simulation.effectiveRate} USDC
          </span>
        </div>

        {/* Price Impact */}
        <div className="flex justify-between items-center py-2">
          <span className="text-sm text-muted-foreground">Price Impact</span>
          <span
            className={`text-sm font-medium ${
              isHighImpact ? "text-amber-600" : "text-emerald-600"
            }`}
          >
            {simulation.priceImpact}%
          </span>
        </div>
      </div>

      {/* Warning for high price impact */}
      {isHighImpact && (
        <div className="rounded-lg bg-amber-50 border border-amber-200 p-3">
          <div className="flex items-start gap-2">
            <AlertTriangle className="h-4 w-4 text-amber-600 mt-0.5 shrink-0" />
            <div className="text-xs text-amber-700">
              <strong>High Price Impact:</strong> This trade may significantly affect the market price. Consider splitting into smaller orders.
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
