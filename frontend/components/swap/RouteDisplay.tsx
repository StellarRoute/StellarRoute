import { ArrowDown, ArrowRight, ChevronDown, Info, AlertCircle } from 'lucide-react';
import { useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { useVirtualWindow } from '@/hooks/useVirtualWindow';
import { useReducedMotion } from '@/hooks/useReducedMotion';
import { cn } from '@/lib/utils';

import { ConfidenceIndicator } from './ConfidenceIndicator';
import { RouteDisplaySkeleton } from './RouteDisplaySkeleton';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { VenueBadgeLegend } from './VenueBadgeLegend';

export interface AlternativeRoute {
  id: string;
  venue: string;
  expectedAmount: string;
  score?: number;
  impactBps?: number;
  hopCount?: number;
  hops?: Array<{
    id: string;
    fromAsset: string;
    toAsset: string;
    venue: string;
    fee: string;
  }>;
}

interface RouteDisplayProps {
  amountOut: string;
  /** Route confidence score (0-100) */
  confidenceScore?: number;
  /** Market volatility level */
  volatility?: 'high' | 'medium' | 'low';
  /** Show loading skeleton */
  isLoading?: boolean;
  /** Show routes data is being fetched */
  isRoutesLoading?: boolean;
  /** Routes fetch error */
  routesError?: string | null;
  /** Optional alternative route fixture data */
  alternativeRoutes?: AlternativeRoute[];
  /** Callback when an alternative route is selected */
  onSelect?: (route: AlternativeRoute) => void;
  /** Show extended route diagnostics for expert mode */
  extendedRouteDetails?: boolean;
}

const ROUTE_VIRTUALIZATION_THRESHOLD = 8;
const ROUTE_ROW_HEIGHT = 44;
const ROUTE_OVERSCAN = 2;

function getVenueBadgeClass(venue: string): string {
  const lower = venue.toLowerCase();
  if (lower.includes('sdex')) {
    return 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200 border-transparent';
  }
  if (lower.includes('amm') || lower.includes('pool')) {
    return 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200 border-transparent';
  }
  return 'bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-200 border-transparent';
}

function getVenueLabel(venue: string): string {
  const lower = venue.toLowerCase();
  if (lower.includes('sdex')) return 'SDEX';
  if (lower.includes('amm')) return 'AMM';
  if (lower.includes('pool')) return 'AMM';
  return 'Hybrid';
}

function formatDelta(bestAmount: number, routeAmount: number): string {
  if (bestAmount === 0) return '—';
  const pct = ((routeAmount - bestAmount) / bestAmount) * 100;
  const sign = pct >= 0 ? '+' : '';
  return `${sign}${pct.toFixed(2)}%`;
}

function buildAlternativeRoutes(amountOut: string): AlternativeRoute[] {
  const venues = ['AQUA Pool', 'SDEX', 'Blend Pool', 'Phoenix AMM'];
  const baseAmount = Number.parseFloat(amountOut || '0');

  return venues.map((venue, index) => ({
    id: `route-${index}`,
    venue,
    expectedAmount: `≈ ${(baseAmount * (0.995 - index * 0.0015)).toFixed(4)}`,
    score: Math.max(0, 94.5 - index * 3),
    impactBps: 30 + index * 5,
    hopCount: index % 2 === 0 ? 1 : 2,
    hops:
      index % 2 === 0
        ? [
            {
              id: `${index}-0`,
              fromAsset: 'XLM',
              toAsset: 'USDC',
              venue,
              fee: '0.00001 XLM',
            },
          ]
        : [
            {
              id: `${index}-0`,
              fromAsset: 'XLM',
              toAsset: 'AQUA',
              venue: 'SDEX',
              fee: '0.00001 XLM',
            },
            {
              id: `${index}-1`,
              fromAsset: 'AQUA',
              toAsset: 'USDC',
              venue,
              fee: '0.00002 XLM',
            },
          ],
  }));
}

function parseFeeToNumber(fee: string): number {
  const numeric = Number.parseFloat(fee);
  return Number.isFinite(numeric) ? numeric : 0;
}

function formatNetworkFeeFromHops(hops: AlternativeRoute['hops']): string {
  const total = (hops ?? []).reduce(
    (sum, hop) => sum + parseFeeToNumber(hop.fee),
    0
  );
  return `${total.toFixed(5)} XLM`;
}

function AlternativeRouteButton({
  route,
  isSelected = false,
  onSelect,
  prefersReducedMotion = false,
  bestAmount,
}: {
  route: AlternativeRoute;
  isSelected?: boolean;
  onSelect?: (route: AlternativeRoute) => void;
  prefersReducedMotion?: boolean;
  bestAmount: number;
}) {
  const routeAmount = Number.parseFloat(route.expectedAmount.replace(/^≈\s*/, ''));
  const delta = formatDelta(bestAmount, routeAmount);
  const isBest = delta === '+0.00%' || delta === '—';

  return (
    <button
      type="button"
      data-testid={`alternative-route-${route.id}`}
      aria-pressed={isSelected}
      data-selected={isSelected ? 'true' : undefined}
      className={cn(
        'w-full flex flex-wrap items-center justify-between p-1.5 -mx-1 rounded hover:bg-muted/50 focus:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20 gap-1 text-left',
        !prefersReducedMotion && 'transition-all duration-150 active:scale-[0.99]',
        isSelected
          ? 'opacity-100 ring-2 ring-primary/40 bg-muted/50'
          : 'opacity-60 hover:opacity-100 focus:opacity-100'
      )}
      onClick={() => onSelect?.(route)}
    >
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground min-w-0">
        <span className="border border-border/50 rounded bg-background px-1.5 py-0.5 text-[10px] font-semibold">
          {getVenueLabel(route.venue)}
        </span>
        <span className="text-[10px] text-muted-foreground truncate max-w-[80px]">
          {route.venue}
        </span>
        {route.hopCount !== undefined && (
          <span className="text-[10px] text-muted-foreground">
            {route.hopCount}h
          </span>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Badge
          variant="secondary"
          className={cn(
            'text-[10px] font-mono px-1.5 py-0',
            isBest
              ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
              : delta.startsWith('-')
                ? 'bg-red-500/10 text-red-600 dark:text-red-400'
                : 'bg-muted text-muted-foreground'
          )}
        >
          {delta}
        </Badge>
        <div className="flex flex-col items-end gap-0.5">
          <span className="text-xs font-medium text-muted-foreground">
            {route.expectedAmount}
          </span>
        </div>
      </div>
    </button>
  );
}

export function RouteDisplay({
  amountOut,
  confidenceScore = 85,
  volatility = 'low',
  isLoading = false,
  isRoutesLoading = false,
  routesError = null,
  alternativeRoutes,
  onSelect,
  extendedRouteDetails = false,
}: RouteDisplayProps) {
  const [showDetails, setShowDetails] = useState(false);
  const [selectedRouteId, setSelectedRouteId] = useState<string | null>(null);
  const prefersReducedMotion = useReducedMotion();
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
  const selectedRoute =
    routes.find((route) => route.id === selectedRouteId) ?? routes[0] ?? null;
  const selectedRouteHops = selectedRoute?.hops ?? [];
  const totalRouteFee = selectedRouteHops.reduce(
    (sum, hop) => sum + parseFeeToNumber(hop.fee),
    0
  );
  const bestAmount = routes.length > 0
    ? Math.max(...routes.map((r) => Number.parseFloat(r.expectedAmount.replace(/^≈\s*/, '') || '0')))
    : 0;

  if (isLoading) {
    return <RouteDisplaySkeleton />;
  }

  return (
    <div
      data-testid="route-display"
      className={cn(
        'rounded-xl border border-border/50 p-4 space-y-4 focus-within:ring-2 focus-within:ring-primary/20',
        !prefersReducedMotion && 'transition-all duration-200 hover:border-border hover:shadow-sm'
      )}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-medium">Best Route</h4>
          <Popover>
            <PopoverTrigger asChild>
              <button
                type="button"
                className="flex items-center justify-center h-5 w-5 rounded-md hover:bg-muted focus:bg-muted focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all text-muted-foreground hover:text-foreground cursor-pointer"
                aria-label="Route and venue badge information legend"
              >
                <Info className="h-4 w-4" />
              </button>
            </PopoverTrigger>
            <PopoverContent className="w-80 p-4 border border-border/80 bg-popover text-popover-foreground shadow-lg" align="start" side="bottom" sideOffset={8}>
              <VenueBadgeLegend />
            </PopoverContent>
          </Popover>
        </div>
        <div className="flex items-center gap-2">
          <ConfidenceIndicator
            score={confidenceScore}
            volatility={volatility}
          />
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
            className={cn(
              'min-h-[44px] min-w-[44px] flex items-center justify-center rounded-md hover:bg-muted/50 focus:bg-muted/50 focus:outline-none focus:ring-2 focus:ring-primary/20',
              !prefersReducedMotion && 'transition-all duration-150 active:scale-95'
            )}
          >
            <ChevronDown
              className={cn(
                'h-4 w-4 text-muted-foreground',
                !prefersReducedMotion && 'transition-transform duration-200',
                showDetails && 'rotate-180'
              )}
            />
          </button>
        </div>
      </div>

      <div className={cn(
        'flex flex-col sm:flex-row items-center bg-muted/50 rounded-lg p-3 overflow-hidden gap-1 sm:gap-0 sm:justify-between',
        !prefersReducedMotion && 'transition-colors duration-150 hover:bg-muted/70'
      )}>
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
        {isRoutesLoading ? (
          <div className="py-3 text-center">
            <div className="h-3 w-3 animate-spin rounded-full border border-primary border-r-transparent mx-auto" />
            <p className="text-xs text-muted-foreground mt-2">Loading routes...</p>
          </div>
        ) : routesError ? (
          <div
            data-testid="routes-error"
            className="flex items-center gap-2 py-2 text-xs text-red-500"
          >
            <AlertCircle className="h-3 w-3 flex-shrink-0" />
            <span>{routesError}</span>
          </div>
        ) : routes.length === 0 ? (
          <p
            data-testid="routes-empty"
            className="text-xs text-muted-foreground py-2 text-center"
          >
            No alternative routes available
          </p>
        ) : (
          <div
            ref={scrollRef}
            data-testid="alternative-routes-scroll"
            className={shouldVirtualize ? 'max-h-44 overflow-auto pr-1' : ''}
          >
            {shouldVirtualize ? (
              <div
                style={{
                  height: virtualWindow.totalHeight,
                  position: 'relative',
                }}
              >
                {visibleRoutes.map((route, index) => {
                  const absoluteIndex = virtualWindow.startIndex + index;
                  return (
                    <div
                      key={route.id}
                      style={{
                        position: 'absolute',
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
                        prefersReducedMotion={prefersReducedMotion}
                        bestAmount={bestAmount}
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
                    prefersReducedMotion={prefersReducedMotion}
                    bestAmount={bestAmount}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {showDetails && selectedRoute && (
        <div
          aria-label="Route detail drawer"
          className="rounded-lg border border-border/50 bg-muted/30 p-3 space-y-3"
        >
          <div className="flex items-center justify-between">
            <h5 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Per-hop route details
            </h5>
            <span className="text-xs text-muted-foreground">
              {selectedRouteHops.length} hop
              {selectedRouteHops.length === 1 ? '' : 's'}
            </span>
          </div>

          {selectedRouteHops.length > 0 ? (
            <div aria-label="Per-hop route details" className="space-y-2">
              {selectedRouteHops.map((hop, index) => (
                <div
                  key={hop.id}
                  className="rounded-md border border-border/40 bg-background/70 px-3 py-2"
                >
                  <div className="flex items-center justify-between text-xs font-medium">
                    <span>
                      Hop {index + 1}: {hop.fromAsset} {'->'} {hop.toAsset}
                    </span>
                    <span className="text-muted-foreground">{hop.venue}</span>
                  </div>
                  <div className="mt-1 flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>Fee</span>
                    <span>{hop.fee}</span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs text-muted-foreground">
              No hop breakdown available for this route.
            </p>
          )}

          <div className="rounded-md border border-border/40 bg-background/70 px-3 py-2">
            <div className="flex items-center justify-between text-xs font-medium">
              <span>Estimated total fees</span>
              <span>{totalRouteFee.toFixed(5)} XLM</span>
            </div>
            <div className="mt-1 flex items-center justify-between text-[11px] text-muted-foreground">
              <span>Route venue</span>
              <span>{selectedRoute.venue}</span>
            </div>
          </div>
        </div>
      )}

      {extendedRouteDetails && selectedRoute && (
        <div
          data-testid="extended-diagnostics"
          className={cn(
            'rounded-lg border border-amber-500/20 bg-amber-500/5 p-3.5 space-y-2.5 font-mono text-[10px] text-amber-600 dark:text-amber-400',
            !prefersReducedMotion && 'animate-in fade-in slide-in-from-bottom-2 duration-300'
          )}
        >
          <div className="flex items-center justify-between border-b border-amber-500/10 pb-1.5">
            <span className="font-bold uppercase tracking-wider">Extended Diagnostics</span>
            <span className="bg-amber-500/10 px-1.5 py-0.5 rounded text-[9px] font-semibold text-amber-600 dark:text-amber-400">RAW_MODE</span>
          </div>
          <div className="space-y-1 text-muted-foreground dark:text-amber-400/80">
            <div><span className="text-amber-600 dark:text-amber-500">path_id:</span> {selectedRoute.id}</div>
            <div><span className="text-amber-600 dark:text-amber-500">venue_agent:</span> {selectedRoute.venue}</div>
            <div><span className="text-amber-600 dark:text-amber-500">sim_status:</span> <span className="text-emerald-500">SUCCESS_OK</span></div>
            <div><span className="text-amber-600 dark:text-amber-500">gas_pool:</span> {totalRouteFee.toFixed(5)} XLM</div>
            <div><span className="text-amber-600 dark:text-amber-500">hops_count:</span> {selectedRouteHops.length}</div>
            <div>
              <span className="text-amber-600 dark:text-amber-500">raw_hops:</span>
              <pre className="mt-1 pl-2 border-l border-amber-500/10 text-[9px] leading-relaxed overflow-x-auto">
                {JSON.stringify(
                  selectedRouteHops.map((h) => ({
                    hop_id: h.id,
                    from: h.fromAsset,
                    to: h.toAsset,
                    pool_venue: h.venue,
                    est_fee: h.fee
                  })),
                  null,
                  2
                )}
              </pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
