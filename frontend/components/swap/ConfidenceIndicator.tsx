"use client";

import { AlertTriangle, TrendingUp, TrendingDown, Minus } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export type ConfidenceLevel = "high" | "medium" | "low";

interface ConfidenceIndicatorProps {
  /** Confidence score from 0-100 */
  score: number;
  /** Volatility level (optional) */
  volatility?: "high" | "medium" | "low";
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

/**
 * Confidence indicator component for route stability assessment
 * Displays low/medium/high confidence with clear legend
 * Shows high-volatility warnings when applicable
 */
export function ConfidenceIndicator({
  score,
  volatility,
}: ConfidenceIndicatorProps) {
  const level = getConfidenceLevel(score);
  const isHighVolatility = volatility === "high";

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
                variant="outline"
                className="text-xs bg-warning/10 text-warning border-warning/20 flex items-center gap-1 animate-pulse"
              >
                <AlertTriangle className="h-3 w-3" />
                Volatile
              </Badge>
            )}
          </div>
        </TooltipTrigger>
        <TooltipContent side="top" className="max-w-[250px]">
          <div className="space-y-2">
            <p className="font-medium text-sm">Route Confidence: {score}%</p>
            <div className="text-xs space-y-1">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-success" />
                <span>High (80-100%): Stable route</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-warning" />
                <span>Medium (50-79%): Moderate stability</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-destructive" />
                <span>Low (&lt;50%): Unstable route</span>
              </div>
            </div>
            {isHighVolatility && (
              <p className="mt-2 border-t border-border pt-2 text-xs text-warning">
                ⚠️ High volatility detected. Route may change frequently.
              </p>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
