'use client';

import { useId, useState } from 'react';
import { SplitRouteData, RouteMetrics } from '@/types/route';
import { PathStep } from '@/types';
import {
  ChevronDown,
  ChevronUp,
  Info,
  ArrowRight,
  Split,
} from 'lucide-react';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import {
  describeTradeRoute,
  getAssetCode,
  parseSource,
} from '@/lib/route-helpers';

interface SplitRouteVisualizationProps {
  splitRoute: SplitRouteData;
  metrics?: RouteMetrics;
  isLoading?: boolean;
  error?: string;
  className?: string;
}

function PathVisualization({
  steps,
  percentage,
  pathIndex,
}: {
  steps: PathStep[];
  percentage: number;
  pathIndex: number;
}) {
  if (steps.length === 0) return null;

  const sourceAsset = steps[0].from_asset;
  const destAsset = steps[steps.length - 1].to_asset;
  const sourceCode = getAssetCode(sourceAsset);
  const destCode = getAssetCode(destAsset);
  const pathSummary = describeTradeRoute(steps);

  return (
    <section
      className="relative p-3 sm:p-4 border rounded-lg bg-muted/20"
      aria-label={`Path ${pathIndex + 1}, ${percentage}% of trade. ${pathSummary}`}
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between mb-3">
        <div className="flex flex-wrap items-center gap-2 min-w-0">
          <Badge variant="outline" className="text-xs shrink-0">
            Path {pathIndex + 1}
          </Badge>
          <span className="text-xs text-muted-foreground truncate">
            {sourceCode} → {destCode}
          </span>
        </div>
        <Badge className="bg-blue-500 text-white shrink-0 self-start sm:self-auto">
          {percentage}% of trade
        </Badge>
      </div>

      <div
        className="flex items-stretch gap-0 overflow-x-auto pb-2 -mx-1 px-1 scroll-smooth snap-x snap-mandatory"
        tabIndex={0}
        role="list"
        aria-label={`Hops for path ${pathIndex + 1}`}
      >
        {steps.map((step, index) => {
          const { isSDEX, poolName } = parseSource(step.source);
          const fromCode = getAssetCode(step.from_asset);
          const toCode = getAssetCode(step.to_asset);
          const venue = isSDEX ? 'SDEX' : poolName || 'AMM';
          const depth = step.liquidity_depth;
          const feeBps = step.fee_bps;

          return (
            <div
              key={index}
              role="listitem"
              className="flex items-center gap-1 sm:gap-2 shrink-0 snap-start"
            >
              {index === 0 && (
                <div
                  className="flex flex-col items-center gap-1"
                  role="group"
                  aria-label={`Start: ${fromCode}`}
                >
                  <div
                    className="size-11 sm:size-12 rounded-full border-2 border-blue-500 flex items-center justify-center bg-background shrink-0"
                    aria-hidden="true"
                  >
                    <span className="text-xs font-semibold">
                      {fromCode.substring(0, 2)}
                    </span>
                  </div>
                  <span
                    className="text-xs max-w-[4rem] truncate"
                    aria-hidden="true"
                  >
                    {fromCode}
                  </span>
                </div>
              )}

              <div className="flex flex-col items-center px-1 sm:px-2" aria-hidden="true">
                <ArrowRight className="w-4 h-4 text-muted-foreground" />
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Badge
                        variant="secondary"
                        className={cn(
                          'text-xs mt-1 max-w-[5rem] truncate cursor-help',
                          isSDEX
                            ? 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200'
                            : 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200'
                        )}
                        tabIndex={0}
                      >
                        {venue}
                      </Badge>
                    </TooltipTrigger>
                    <TooltipContent className="p-3">
                      <div className="space-y-1.5 text-xs">
                        <p className="font-semibold">{venue} Details</p>
                        {depth && (
                          <div className="flex justify-between gap-4">
                            <span className="text-muted-foreground">Liquidity Depth:</span>
                            <span className="font-medium">{depth}</span>
                          </div>
                        )}
                        {feeBps !== undefined && (
                          <div className="flex justify-between gap-4">
                            <span className="text-muted-foreground">Fee:</span>
                            <span className="font-medium">{(feeBps / 100).toFixed(2)}%</span>
                          </div>
                        )}
                        <div className="flex justify-between gap-4">
                          <span className="text-muted-foreground">Price:</span>
                          <span className="font-medium">{parseFloat(step.price).toFixed(6)}</span>
                        </div>
                      </div>
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>

              <div
                className="flex flex-col items-center gap-1"
                role="group"
                aria-label={
                  index === steps.length - 1
                    ? `Destination: ${toCode}`
                    : `Intermediate: ${toCode}`
                }
              >
                <div
                  className={cn(
                    'size-11 sm:size-12 rounded-full border-2 flex items-center justify-center bg-background shrink-0',
                    index === steps.length - 1
                      ? 'border-green-500'
                      : 'border-neutral-300 dark:border-neutral-600'
                  )}
                  aria-hidden="true"
                >
                  <span className="text-xs font-semibold">
                    {toCode.substring(0, 2)}
                  </span>
                </div>
                <span
                  className="text-xs max-w-[4rem] truncate"
                  aria-hidden="true"
                >
                  {toCode}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function MetricsSummary({ metrics }: { metrics: RouteMetrics }) {
  return (
    <div
      className="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4 p-3 sm:p-4 border rounded-lg bg-muted/10"
      role="group"
      aria-label="Route metrics summary"
    >
      <div>
        <span className="text-xs text-muted-foreground">Total Fees</span>
        <p className="text-sm font-semibold">{metrics.totalFees}</p>
      </div>
      <div>
        <span className="text-xs text-muted-foreground">Price Impact</span>
        <p className="text-sm font-semibold">{metrics.totalPriceImpact}</p>
      </div>
      <div>
        <span className="text-xs text-muted-foreground">Net Output</span>
        <p className="text-sm font-semibold">{metrics.netOutput}</p>
      </div>
      <div>
        <span className="text-xs text-muted-foreground">Avg Rate</span>
        <p className="text-sm font-semibold">{metrics.averageRate}</p>
      </div>
    </div>
  );
}

export function SplitRouteVisualization({
  splitRoute,
  metrics,
  isLoading = false,
  error,
  className,
}: SplitRouteVisualizationProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const titleId = useId();
  const summaryId = useId();
  const detailsPanelId = useId();

  const splitSummary =
    splitRoute?.paths
      .map((p, i) => `Path ${i + 1}: ${describeTradeRoute(p.steps)}`)
      .join(' ') ?? '';

  if (isLoading) {
    return (
      <Card
        className={cn('p-6', className)}
        role="status"
        aria-busy="true"
        aria-label="Loading split route"
      >
        <Skeleton className="w-full h-32 mb-4" />
        <Skeleton className="w-full h-32" />
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={cn('p-6 border-destructive', className)} role="alert">
        <div className="flex items-center gap-2 text-destructive">
          <Info className="w-5 h-5 shrink-0" aria-hidden="true" />
          <span className="text-sm font-medium">{error}</span>
        </div>
      </Card>
    );
  }

  if (!splitRoute || splitRoute.paths.length === 0) {
    return (
      <Card className={cn('p-6', className)} role="status">
        <div className="flex flex-col items-center justify-center gap-2 text-muted-foreground">
          <Info className="w-8 h-8" aria-hidden="true" />
          <span className="text-sm">No route found</span>
        </div>
      </Card>
    );
  }

  const isSplit = splitRoute.paths.length > 1;
  const totalHops = splitRoute.paths.reduce(
    (sum, path) => sum + path.steps.length,
    0
  );

  return (
    <Card className={cn('p-4 sm:p-6', className)}>
      <section aria-labelledby={titleId} aria-describedby={summaryId}>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between mb-4">
          <div className="flex flex-wrap items-center gap-2 min-w-0">
            <h3 id={titleId} className="text-sm font-semibold">
              Trade Route
            </h3>
            {isSplit && (
              <Badge variant="default" className="bg-blue-500 shrink-0">
                <Split className="w-3 h-3 mr-1" aria-hidden="true" />
                Split Route
              </Badge>
            )}
            <Badge variant="outline">
              {totalHops} {totalHops === 1 ? 'Hop' : 'Hops'}
            </Badge>
          </div>
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            aria-expanded={isExpanded}
            aria-controls={detailsPanelId}
            className={cn(
              'inline-flex items-center justify-center gap-1 min-h-11 min-w-11 px-3 py-2 -mr-2 sm:mr-0',
              'text-xs text-muted-foreground hover:text-foreground transition-colors',
              'rounded-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background'
            )}
          >
            <span>{isExpanded ? 'Hide' : 'Show'} detailed breakdown</span>
            {isExpanded ? (
              <ChevronUp className="w-4 h-4 shrink-0" aria-hidden="true" />
            ) : (
              <ChevronDown className="w-4 h-4 shrink-0" aria-hidden="true" />
            )}
          </button>
        </div>

        <p id={summaryId} className="sr-only">
          {splitSummary}
        </p>

        <div className="space-y-3">
          {splitRoute.paths.map((path, index) => (
            <PathVisualization
              key={index}
              steps={path.steps}
              percentage={path.percentage}
              pathIndex={index}
            />
          ))}
        </div>

        {metrics && (
          <div className="mt-4">
            <MetricsSummary metrics={metrics} />
          </div>
        )}

        {isExpanded && (
          <div
            id={detailsPanelId}
            className="mt-6 space-y-4 border-t pt-4"
            role="region"
            aria-label="Detailed per-path hops"
          >
            <h4 className="text-sm font-semibold">Detailed Breakdown</h4>
            {splitRoute.paths.map((path, pathIndex) => (
              <div key={pathIndex} className="space-y-2">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">Path {pathIndex + 1}</Badge>
                  <span className="text-xs text-muted-foreground">
                    {path.percentage}% allocation
                  </span>
                  {path.outputAmount && (
                    <span className="text-xs text-muted-foreground">
                      Output: {path.outputAmount}
                    </span>
                  )}
                </div>
                {path.steps.map((step, stepIndex) => {
                  const { isSDEX, poolName } = parseSource(step.source);
                  const fromCode = getAssetCode(step.from_asset);
                  const toCode = getAssetCode(step.to_asset);

                  return (
                    <div
                      key={stepIndex}
                      className="pl-3 sm:pl-4 p-3 border-l-2 border-muted"
                    >
                      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between mb-1">
                        <span className="text-sm font-medium">
                          Hop {stepIndex + 1}: {fromCode} → {toCode}
                        </span>
                        <Badge
                          variant={isSDEX ? 'default' : 'secondary'}
                          className="text-xs w-fit"
                        >
                          {isSDEX ? 'SDEX' : poolName || 'AMM'}
                        </Badge>
                      </div>
                      <div className="text-xs text-muted-foreground space-y-0.5">
                        <div>Rate: {parseFloat(step.price).toFixed(6)}</div>
                        {step.liquidity_depth && (
                          <div>Depth: {step.liquidity_depth}</div>
                        )}
                        {step.fee_bps !== undefined && (
                          <div>Fee: {(step.fee_bps / 100).toFixed(2)}%</div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        )}
      </section>
    </Card>
  );
}
