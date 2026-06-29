"use client";

import { AlertTriangle, TrendingUp, TrendingDown, Minus } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useReducedMotion } from "@/hooks/useReducedMotion";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export type ConfidenceLevel = "high" | "medium" | "low";

export interface RiskFactor {
  /** Factor name, e.g. "Liquidity Depth" */
  label: string;
  /** Severity: ok = green, warn = amber, bad = red */
  severity: "ok" | "warn" | "bad";
  /** Short description shown in tooltip */
  description: string;
}

interface ConfidenceIndicatorProps {
  /** Confidence score from 0-100 */
  score: number;
  /** Volatility level (optional) */
  volatility?: "high" | "medium" | "low";
  /**
   * Explicit risk factor breakdown. When omitted, a default breakdown is
   * derived from score + volatility so the tooltip always has content.
   */
  riskFactors?: RiskFactor[];
}

/**
 * Determines confidence level based on score
 * - High: score >= 80
 * - Medium: score >= 50
 * - Low: score < 50
 */
function getConfidenceLevel(score: number): ConfidenceLevel {
  if (score >= 80) return "high";
  if (score >= 50) return "medium";
  return "low";
}

/** Build a sensible default factor list when the caller doesn't supply one. */
function defaultRiskFactors(
  score: number,
  volatility?: "high" | "medium" | "low"
): RiskFactor[] {
  const liquiditySeverity: RiskFactor["severity"] =
    score >= 80 ? "ok" : score >= 50 ? "warn" : "bad";
  const freshnessSeverity: RiskFactor["severity"] =
    score >= 70 ? "ok" : score >= 40 ? "warn" : "bad";
  const volatilitySeverity: RiskFactor["severity"] =
    volatility === "high" ? "bad" : volatility === "medium" ? "warn" : "ok";

  return [
    {
      label: "Liquidity Depth",
      severity: liquiditySeverity,
      description:
        liquiditySeverity === "ok"
          ? "Sufficient depth to fill this order with minimal slippage."
          : liquiditySeverity === "warn"
            ? "Moderate depth — larger orders may experience slippage."
            : "Thin liquidity — high slippage risk for this order size.",
    },
    {
      label: "Source Freshness",
      severity: freshnessSeverity,
      description:
        freshnessSeverity === "ok"
          ? "Market data is recent and reliable."
          : freshnessSeverity === "warn"
            ? "Data is slightly stale; price may have shifted."
            : "Stale market data — execution price may differ significantly.",
    },
    {
      label: "Volatility",
      severity: volatilitySeverity,
      description:
        volatilitySeverity === "ok"
          ? "Low volatility — route is stable."
          : volatilitySeverity === "warn"
            ? "Moderate volatility — route may change before execution."
            : "High volatility — route is unstable and may change frequently.",
    },
  ];
}

const SEVERITY_DOT: Record<RiskFactor["severity"], string> = {
  ok: "bg-success",
  warn: "bg-warning",
  bad: "bg-destructive",
};

const SEVERITY_TEXT: Record<RiskFactor["severity"], string> = {
  ok: "text-success",
  warn: "text-warning",
  bad: "text-destructive",
};

/**
 * Confidence indicator component for route stability assessment.
 * Tooltip breaks down risk factors: liquidity depth, volatility, source freshness.
 */
export function ConfidenceIndicator({
  score,
  volatility,
  riskFactors,
}: ConfidenceIndicatorProps) {
  const level = getConfidenceLevel(score);
  const isHighVolatility = volatility === "high";
  const prefersReducedMotion = useReducedMotion();

  const factors = riskFactors ?? defaultRiskFactors(score, volatility);

  const config = {
    high: {
      label: "High",
      className: "bg-success/10 text-success border-success/20",
      icon: TrendingUp,
    },
    medium: {
      label: "Medium",
      className: "bg-warning/10 text-warning border-warning/20",
      icon: Minus,
    },
    low: {
      label: "Low",
      className: "bg-destructive/10 text-destructive border-destructive/20",
      icon: TrendingDown,
    },
  };

  const { label, className, icon: Icon } = config[level];

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="flex items-center gap-1.5">
            <Badge
              variant="outline"
              className={`text-xs ${className} flex items-center gap-1`}
            >
              <Icon className="h-3 w-3" />
              {label} Confidence
            </Badge>
            {isHighVolatility && (
              <Badge
                data-testid="volatile-badge"
                variant="outline"
                className={cn(
                  "text-xs bg-warning/10 text-warning border-warning/20 flex items-center gap-1",
                  !prefersReducedMotion && "animate-pulse"
                )}
              >
                <AlertTriangle className="h-3 w-3" />
                Volatile
              </Badge>
            )}
            {/* Visually-hidden risk factor summary for screen readers and tests */}
            <span className="sr-only" aria-label="Risk factor breakdown">
              <span data-testid="risk-factors">
                {factors.map((factor) => (
                  <span
                    key={factor.label}
                    data-testid={`risk-factor-${factor.label.toLowerCase().replace(/\s+/g, "-")}`}
                  >
                    {factor.label}: {factor.description}
                  </span>
                ))}
              </span>
            </span>
          </div>
        </TooltipTrigger>
        <TooltipContent side="top" className="max-w-[280px]">
          <div className="space-y-2">
            <p className="font-medium text-sm">Route Confidence: {score}%</p>

            {/* Risk factor breakdown */}
            <div
              className="space-y-1.5"
              aria-label="Risk factor breakdown"
            >
              {factors.map((factor) => (
                <div
                  key={factor.label}
                  className="flex items-start gap-2"
                >
                  <span
                    className={cn(
                      "mt-0.5 h-2 w-2 shrink-0 rounded-full",
                      SEVERITY_DOT[factor.severity]
                    )}
                    aria-hidden="true"
                  />
                  <div className="min-w-0">
                    <span
                      className={cn(
                        "text-xs font-semibold",
                        SEVERITY_TEXT[factor.severity]
                      )}
                    >
                      {factor.label}
                    </span>
                    <p className="text-[11px] text-muted-foreground leading-snug">
                      {factor.description}
                    </p>
                  </div>
                </div>
              ))}
            </div>

            {/* Legend */}
            <div className="border-t border-border pt-1.5 text-xs space-y-0.5">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-success" />
                <span>High (80–100%): Stable route</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-warning" />
                <span>Medium (50–79%): Moderate stability</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-destructive" />
                <span>Low (&lt;50%): Unstable route</span>
              </div>
            </div>

            {isHighVolatility && (
              <p className="border-t border-border pt-1.5 text-xs text-warning">
                ⚠️ High volatility detected. Route may change frequently.
              </p>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
