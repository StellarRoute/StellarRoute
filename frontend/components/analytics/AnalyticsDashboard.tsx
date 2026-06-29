"use client";

import { Activity, Database, RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { ViewState } from "@/components/shared/ViewState";
import { useCacheMetrics, usePoolStats } from "@/hooks/useApi";
import type { PoolStats } from "@/types";
import { cn } from "@/lib/utils";

function formatPercent(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

function PoolStatsCard({ label, stats }: { label: string; stats: PoolStats }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-base">{label}</CardTitle>
        <CardDescription>
          {stats.in_use} in use · {stats.idle} idle · max {stats.max_connections}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-end justify-between gap-4">
          <div>
            <p className="text-3xl font-bold tabular-nums">
              {formatPercent(stats.utilisation)}
            </p>
            <p className="text-sm text-muted-foreground">utilisation</p>
          </div>
          <div
            className="h-2 flex-1 max-w-[160px] rounded-full bg-muted overflow-hidden"
            role="progressbar"
            aria-valuenow={Math.round(stats.utilisation * 100)}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label={`${label} pool utilisation`}
          >
            <div
              className={cn(
                "h-full rounded-full transition-all",
                stats.utilisation > 0.8 ? "bg-amber-500" : "bg-primary",
              )}
              style={{ width: `${Math.min(stats.utilisation * 100, 100)}%` }}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export function AnalyticsDashboard() {
  const {
    data: cacheMetrics,
    loading: cacheLoading,
    error: cacheError,
    refresh: refreshCache,
  } = useCacheMetrics();
  const {
    data: poolStats,
    loading: poolLoading,
    error: poolError,
    refresh: refreshPool,
  } = usePoolStats();

  const loading = cacheLoading || poolLoading;
  const hasError = cacheError || poolError;

  const handleRefresh = () => {
    refreshCache();
    refreshPool();
  };

  return (
    <div className="w-full px-4 py-8 sm:px-6 lg:px-8 space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-3xl font-bold">Analytics</h1>
          <p className="text-muted-foreground">
            Platform cache and database metrics from the StellarRoute API.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          onClick={handleRefresh}
          disabled={loading}
          aria-label="Refresh analytics metrics"
        >
          <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
          Refresh
        </Button>
      </div>

      {loading && !cacheMetrics && !poolStats ? (
        <ViewState
          variant="loading"
          title="Loading metrics"
          description="Fetching cache and pool statistics from the API."
        />
      ) : hasError && !cacheMetrics && !poolStats ? (
        <ViewState
          variant="error"
          title="Metrics unavailable"
          description="Could not load analytics data. The API may be offline or metrics endpoints are disabled."
        />
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <Card>
            <CardHeader className="pb-2">
              <div className="flex items-center gap-2">
                <Activity className="h-5 w-5 text-primary" aria-hidden="true" />
                <CardTitle className="text-base">Quote cache</CardTitle>
              </div>
              <CardDescription>Hit ratio and staleness counters</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {cacheMetrics ? (
                <>
                  <div>
                    <p className="text-3xl font-bold tabular-nums">
                      {formatPercent(cacheMetrics.hit_ratio)}
                    </p>
                    <p className="text-sm text-muted-foreground">cache hit ratio</p>
                  </div>
                  <dl className="grid grid-cols-2 gap-3 text-sm">
                    <div>
                      <dt className="text-muted-foreground">Hits</dt>
                      <dd className="font-medium tabular-nums">
                        {cacheMetrics.quote_hits.toLocaleString()}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Misses</dt>
                      <dd className="font-medium tabular-nums">
                        {cacheMetrics.quote_misses.toLocaleString()}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Stale rejections</dt>
                      <dd className="font-medium tabular-nums">
                        {cacheMetrics.stale_quote_rejections.toLocaleString()}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Stale inputs excluded</dt>
                      <dd className="font-medium tabular-nums">
                        {cacheMetrics.stale_inputs_excluded.toLocaleString()}
                      </dd>
                    </div>
                  </dl>
                </>
              ) : (
                <p className="text-sm text-muted-foreground">Cache metrics unavailable</p>
              )}
            </CardContent>
          </Card>

          {poolStats ? (
            <>
              <PoolStatsCard label="Primary pool" stats={poolStats.primary} />
              {poolStats.replica ? (
                <PoolStatsCard label="Replica pool" stats={poolStats.replica} />
              ) : (
                <Card>
                  <CardHeader className="pb-2">
                    <div className="flex items-center gap-2">
                      <Database className="h-5 w-5 text-muted-foreground" aria-hidden="true" />
                      <CardTitle className="text-base">Replica pool</CardTitle>
                    </div>
                    <CardDescription>Read replica not configured</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-muted-foreground">
                      No replica pool metrics are exposed for this deployment.
                    </p>
                  </CardContent>
                </Card>
              )}
            </>
          ) : (
            <Card className="md:col-span-2">
              <CardHeader className="pb-2">
                <div className="flex items-center gap-2">
                  <Database className="h-5 w-5 text-muted-foreground" aria-hidden="true" />
                  <CardTitle className="text-base">Database pools</CardTitle>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">Pool metrics unavailable</p>
              </CardContent>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}
