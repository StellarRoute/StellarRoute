import { ArrowDown, ArrowRight, ChevronDown, Info } from "lucide-react";
import { useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";

import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { RouteDisplaySkeleton } from "./RouteDisplaySkeleton";

export interface AlternativeRoute {
  id: string;
  venue: string;
  expectedAmount: string;
}

interface RouteDisplayProps {
  amountOut: string;
  /** Route confidence score (0-100) */
  confidenceScore?: number;
  /** Market volatility level */
  volatility?: "high" | "medium" | "low";
  /** Show loading skeleton */
  isLoading?: boolean;
  /** Optional alternative route fixture data */
  alternativeRoutes?: AlternativeRoute[];
  /** Callback when an alternative route is selected */
  onSelect?: (route: AlternativeRoute) => void;
}

const ROUTE_VIRTUALIZATION_THRESHOLD = 8;
const ROUTE_ROW_HEIGHT = 44;
const ROUTE_OVERSCAN = 2;

function buildAlternativeRoutes(amountOut: string): AlternativeRoute[] {
  const venues = ["AQUA Pool", "SDEX", "Blend Pool", "Phoenix AMM"];
  const baseAmount = Number.parseFloat(amountOut || "0");

  return venues.map((venue, index) => ({
    id: `route-${index}`,
    venue,
    expectedAmount: `≈ ${(baseAmount * (0.995 - index * 0.0015)).toFixed(4)}`,
  }));
}

function AlternativeRouteButton({
  route,
  isSelected = false,
  onSelect,
}: {
  route: AlternativeRoute;
  isSelected?: boolean;
  onSelect?: (route: AlternativeRoute) => void;
}) {
  return (
    <button
      type="button"
      data-testid={`alternative-route-${route.id}`}
      aria-pressed={isSelected}
      data-selected={isSelected ? "true" : undefined}
      className={`w-full flex flex-wrap items-center justify-between transition-all duration-150 p-1 -mx-1 rounded hover:bg-muted/50 focus:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20 gap-1 text-left active:scale-[0.99] ${
        isSelected
          ? "opacity-100 ring-2 ring-primary/40 bg-muted/50"
          : "opacity-60 hover:opacity-100 focus:opacity-100"
      }`}
      onClick={() => onSelect?.(route)}
    >
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <span className="font-medium">XLM</span>
        <ArrowRight className="h-3 w-3" />
        <span className="border border-border/50 rounded bg-background px-1.5 py-0.5 text-[10px]">
          {route.venue}
        </span>
        <ArrowRight className="h-3 w-3" />
        <span className="font-medium">USDC</span>
      </div>
      <span className="text-xs font-medium text-muted-foreground">
        {route.expectedAmount}
      </span>
    </button>
  );
}

export function RouteDisplay({
  amountOut,
  confidenceScore = 85,
  volatility = "low",
  isLoading = false,
  alternativeRoutes,
  onSelect,
}: RouteDisplayProps) {
  const [showDetails, setShowDetails] = useState(false);
  const [selectedRouteId, setSelectedRouteId] = useState<string | null>(null);
  const routes = alternativeRoutes ?? buildAlternativeRoutes(amountOut);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const handleSelect = (route: AlternativeRoute) => {
    setSelectedRouteId(route.id);
    onSelect?.(route);
  };
  const shouldVirtualize = routes.length > ROUTE_VIRTUALIZATION_THRESHOLD;
  const virtualWindow = useVirtualWindow({
    containerRef: scrollRef,
    itemCount: routes.length,
    itemHeight: ROUTE_ROW_HEIGHT,
    overscan: ROUTE_OVERSCAN,
    enabled: shouldVirtualize,
    defaultViewportHeight: ROUTE_ROW_HEIGHT * 4,
  });

  const visibleRoutes = shouldVirtualize
    ? routes.slice(virtualWindow.startIndex, virtualWindow.endIndex)
    : routes;

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
          <Badge
            variant="secondary"
            className="text-xs bg-emerald-500/10 text-emerald-600 hover:bg-emerald-500/20 border-emerald-500/20 transition-colors"
          >
            Optimal
          </Badge>
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

      <div className="flex flex-col sm:flex-row items-center bg-muted/50 rounded-lg p-3 overflow-hidden gap-1 sm:gap-0 sm:justify-between transition-colors duration-150 hover:bg-muted/70">
        <div className="flex flex-col flex-shrink-0 min-w-[40px] items-center sm:items-start">
          <span className="text-xs font-semibold">XLM</span>
          <span className="text-[10px] text-muted-foreground leading-none">
            Stellar
          </span>
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
          <span
            className="text-[10px] text-muted-foreground truncate max-w-[80px]"
            title={`${amountOut} expected`}
          >
            {amountOut} exp.
          </span>
        </div>
      </div>

      <div className="pt-3 border-t border-border/50 overflow-x-hidden">
        <h4 className="text-[11px] font-medium text-muted-foreground mb-2 uppercase tracking-wider">
          Alternative Routes
        </h4>
        <div
          ref={scrollRef}
          data-testid="alternative-routes-scroll"
          className={shouldVirtualize ? "max-h-44 overflow-auto pr-1" : ""}
        >
          {shouldVirtualize ? (
            <div
              style={{
                height: virtualWindow.totalHeight,
                position: "relative",
              }}
            >
              {visibleRoutes.map((route, index) => {
                const absoluteIndex = virtualWindow.startIndex + index;
                return (
                  <div
                    key={route.id}
                    style={{
                      position: "absolute",
                      top: absoluteIndex * ROUTE_ROW_HEIGHT,
                      left: 0,
                      right: 0,
                      height: ROUTE_ROW_HEIGHT,
                    }}
                  >
                    <AlternativeRouteButton
                      route={route}
                      isSelected={selectedRouteId === route.id}
                      onSelect={handleSelect}
                    />
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="space-y-1">
              {visibleRoutes.map((route) => (
                <AlternativeRouteButton
                  key={route.id}
                  route={route}
                  isSelected={selectedRouteId === route.id}
                  onSelect={handleSelect}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
