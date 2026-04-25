'use client';

import { useEffect, useId, useState } from 'react';
import { PathStep } from '@/types';
import { ChevronDown, ChevronUp, Info, ArrowRight } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { AssetIcon } from '@/components/shared/AssetIcon';
import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  describeTradeRoute,
  getAssetCode,
  parseSource,
} from '@/lib/route-helpers';

interface RouteVisualizationProps {
  path: PathStep[];
  isLoading?: boolean;
  error?: string;
  className?: string;
  breakdown?: {
    totalFees?: string;
    priceImpact?: string;
    hops?: number;
  };
}

interface RouteNode {
  asset: PathStep['from_asset'];
  amount?: string;
  isSource: boolean;
  isDestination: boolean;
}

interface RouteEdge {
  step: PathStep;
  isSDEX: boolean;
  poolName?: string;
}

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const sync = () => setReduced(mq.matches);
    sync();
    mq.addEventListener('change', sync);
    return () => mq.removeEventListener('change', sync);
  }, []);

  return reduced;
}

function buildRouteGraph(path: PathStep[]): {
  nodes: RouteNode[];
  edges: RouteEdge[];
} {
  if (path.length === 0) {
    return { nodes: [], edges: [] };
  }

  const nodes: RouteNode[] = [];
  const edges: RouteEdge[] = [];

  // First node (source)
  nodes.push({
    asset: path[0].from_asset,
    isSource: true,
    isDestination: false,
  });

  // Process each step
  path.forEach((step, index) => {
    const { isSDEX, poolName } = parseSource(step.source);

    edges.push({
      step,
      isSDEX,
      poolName,
    });

    nodes.push({
      asset: step.to_asset,
      isSource: false,
      isDestination: index === path.length - 1,
    });
  });

  return { nodes, edges };
}

// ---------------------------------------------------------------------------
// Sub-Components
// ---------------------------------------------------------------------------

function RouteNodeComponent({
  node,
  label: labelProp,
}: {
  node: RouteNode;
  /** Accessible name; defaults from asset code when omitted */
  label?: string;
}) {
  const assetCode = getAssetCode(node.asset);
  const label = labelProp ?? `Route node ${assetCode}`;

  return (
    <div
      className="flex flex-col items-center gap-2 min-w-[5.5rem] sm:min-w-[6rem]"
      role="group"
      aria-label={label}
    >
      <div
        className={cn(
          'flex items-center justify-center size-12 sm:size-14 rounded-full border-2 bg-background shrink-0',
          node.isSource && 'border-blue-500 ring-2 ring-blue-500/20',
          node.isDestination && 'border-green-500 ring-2 ring-green-500/20',
          !node.isSource && !node.isDestination && 'border-neutral-300 dark:border-neutral-600'
        )}
        aria-hidden="true"
      >
        <AssetIcon
          symbol={assetCode}
          className="size-full border-0 bg-transparent text-sm"
          fallbackClassName="font-semibold"
          maxCharacters={3}
        />
      </div>
      <span
        className="text-xs font-medium text-center max-w-[6rem] truncate"
        aria-hidden="true"
      >
        {assetCode}
      </span>
      {node.amount && (
        <span className="text-xs text-muted-foreground" aria-hidden="true">
          {node.amount}
        </span>
      )}
    </div>
  );
}

function RouteEdgeComponent({
  edge,
  showAnimation,
}: {
  edge: RouteEdge;
  showAnimation: boolean;
}) {
  return (
    <div
      className="flex flex-col items-center justify-center px-2 sm:px-4 relative min-w-[4rem] sm:min-w-[5rem]"
      aria-hidden="true"
    >
      <div className="relative w-full h-0.5 overflow-hidden rounded-full">
        <div
          className={cn(
            'absolute inset-0 rounded-full',
            edge.isSDEX ? 'bg-blue-500' : 'bg-purple-500'
          )}
        />
        {showAnimation && (
          <div
            className={cn(
              'absolute inset-0 w-8 h-full opacity-60 animate-flow',
              edge.isSDEX ? 'bg-blue-300' : 'bg-purple-300'
            )}
          />
        )}
      </div>
      <ArrowRight
        className="absolute w-4 h-4 text-muted-foreground"
        aria-hidden="true"
      />
      <Badge
        variant="secondary"
        className={cn(
          'mt-2 text-xs max-w-[5.5rem] truncate',
          edge.isSDEX
            ? 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200'
            : 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200'
        )}
        title={edge.isSDEX ? 'SDEX' : edge.poolName || 'AMM'}
      >
        {edge.isSDEX ? 'SDEX' : edge.poolName || 'AMM'}
      </Badge>
    </div>
  );
}

function RouteVerticalConnector({ edge }: { edge: RouteEdge }) {
  const venue = edge.isSDEX ? 'SDEX' : edge.poolName || 'AMM';

  return (
    <div
      className="flex flex-col items-center py-2 w-full max-w-[12rem]"
      role="presentation"
    >
      <div className="h-6 w-0.5 bg-border rounded-full" aria-hidden="true" />
      <Badge
        variant="secondary"
        className={cn(
          'my-1 text-xs max-w-[min(100%,10rem)] truncate',
          edge.isSDEX
            ? 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200'
            : 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200'
        )}
        title={venue}
      >
        {edge.isSDEX ? 'SDEX' : edge.poolName || 'AMM'}
      </Badge>
      <div className="h-6 w-0.5 bg-border rounded-full" aria-hidden="true" />
    </div>
  );
}

function RouteDetails({ step, index }: { step: PathStep; index: number }) {
  const { isSDEX, poolName } = parseSource(step.source);
  const fromCode = getAssetCode(step.from_asset);
  const toCode = getAssetCode(step.to_asset);

  return (
    <div className="p-3 border rounded-lg bg-muted/30">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between mb-2">
        <span className="text-sm font-medium">
          Hop {index + 1}: {fromCode} → {toCode}
        </span>
        <Badge variant={isSDEX ? 'default' : 'secondary'}>
          {isSDEX ? 'SDEX' : poolName || 'AMM Pool'}
        </Badge>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
        <div>
          <span className="text-muted-foreground">Exchange Rate:</span>
          <p className="font-medium">{parseFloat(step.price).toFixed(6)}</p>
        </div>
        <div>
          <span className="text-muted-foreground">Source:</span>
          <p className="font-medium break-all">{step.source}</p>
        </div>
      </div>
    </div>
  );
}

export function RouteVisualization({
  path,
  isLoading = false,
  error,
  className,
  breakdown,
}: RouteVisualizationProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const reducedMotion = usePrefersReducedMotion();
  const titleId = useId();
  const summaryId = useId();
  const detailsPanelId = useId();
  const showEdgeAnimation = !reducedMotion;

  if (isLoading) {
    return (
      <Card
        className={cn('p-6', className)}
        role="status"
        aria-busy="true"
        aria-label="Loading trade route"
      >
        <div className="flex items-center gap-4 overflow-x-auto pb-2">
          <Skeleton className="size-12 sm:size-14 rounded-full shrink-0" />
          <Skeleton className="w-20 sm:w-24 h-8 shrink-0" />
          <Skeleton className="size-12 sm:size-14 rounded-full shrink-0" />
          <Skeleton className="w-20 sm:w-24 h-8 shrink-0" />
          <Skeleton className="size-12 sm:size-14 rounded-full shrink-0" />
        </div>
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

  if (!path || path.length === 0) {
    return (
      <Card className={cn('p-6', className)} role="status">
        <div className="flex flex-col items-center justify-center gap-2 text-muted-foreground">
          <Info className="w-8 h-8" aria-hidden="true" />
          <span className="text-sm">No route found</span>
        </div>
      </Card>
    );
  }

  const { nodes, edges } = buildRouteGraph(path);
  const hopCount = path.length;
  const routeSummary = describeTradeRoute(path);
  const breakdownHops = breakdown?.hops ?? hopCount;
  const breakdownFees = breakdown?.totalFees ?? 'N/A';
  const breakdownImpact = breakdown?.priceImpact ?? 'N/A';

  return (
    <Card className={cn('p-4 sm:p-6', className)}>
      <section aria-labelledby={titleId} aria-describedby={summaryId}>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between mb-4">
          <div className="flex flex-wrap items-center gap-2 min-w-0">
            <h3 id={titleId} className="text-sm font-semibold">
              Trade Route
            </h3>
            <Badge variant="outline">
              {hopCount} {hopCount === 1 ? 'Hop' : 'Hops'}
            </Badge>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    className="inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    aria-label="Route breakdown details"
                  >
                    <Info className="w-4 h-4" aria-hidden="true" />
                  </button>
                </TooltipTrigger>
                <TooltipContent className="w-56">
                  <div className="space-y-2 text-xs">
                    <p className="font-semibold">Route breakdown</p>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Hops</span>
                      <span>{breakdownHops}</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Est. fees</span>
                      <span>{breakdownFees}</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Price impact</span>
                      <span>{breakdownImpact}</span>
                    </div>
                  </div>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
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
            <span>{isExpanded ? 'Hide' : 'Show'} route details</span>
            {isExpanded ? (
              <ChevronUp className="w-4 h-4 shrink-0" aria-hidden="true" />
            ) : (
              <ChevronDown className="w-4 h-4 shrink-0" aria-hidden="true" />
            )}
          </button>
        </div>

        <p id={summaryId} className="sr-only">
          {routeSummary}
        </p>

        <div
          className="hidden lg:flex items-center justify-start gap-0 overflow-x-auto pb-2 -mx-1 px-1 scroll-smooth snap-x snap-mandatory"
          tabIndex={0}
          role="group"
          aria-label="Route diagram, scroll horizontally on small windows"
        >
          {nodes.map((node, index) => {
            const position =
              index === 0
                ? 'Start'
                : index === nodes.length - 1
                  ? 'Destination'
                  : 'Intermediate';
            return (
              <div
                key={index}
                className="flex items-center snap-start shrink-0"
              >
                <RouteNodeComponent
                  node={node}
                  label={`${position} asset: ${getAssetCode(node.asset)}`}
                />
                {index < edges.length && (
                  <RouteEdgeComponent
                    edge={edges[index]}
                    showAnimation={showEdgeAnimation}
                  />
                )}
              </div>
            );
          })}
        </div>

        <ol
          className="flex lg:hidden flex-col items-center list-none m-0 p-0 gap-0 w-full"
          aria-label="Route steps"
        >
          {nodes.map((node, index) => {
            const position =
              index === 0
                ? 'Start'
                : index === nodes.length - 1
                  ? 'Destination'
                  : 'Intermediate';
            return (
              <li
                key={index}
                className="flex flex-col items-center w-full max-w-md mx-auto"
              >
                <RouteNodeComponent
                  node={node}
                  label={`Step ${index + 1} of ${nodes.length}: ${position}, ${getAssetCode(node.asset)}`}
                />
                {index < edges.length && (
                  <RouteVerticalConnector edge={edges[index]} />
                )}
              </li>
            );
          })}
        </ol>

        {isExpanded && (
          <div
            id={detailsPanelId}
            className="mt-6 space-y-3 border-t pt-4"
            role="region"
            aria-label="Per-hop route details"
          >
            <h4 className="text-sm font-semibold mb-3">Route Details</h4>
            <div className="space-y-3">
              {path.map((step, index) => (
                <RouteDetails key={index} step={step} index={index} />
              ))}
            </div>
          </div>
        )}
      </section>

      <style jsx>{`
        @keyframes flow {
          0% {
            transform: translateX(-100%);
          }
          100% {
            transform: translateX(400%);
          }
        }
        .animate-flow {
          animation: flow 2s linear infinite;
        }
        @media (prefers-reduced-motion: reduce) {
          .animate-flow {
            animation: none;
          }
        }
      `}</style>
    </Card>
  );
}
