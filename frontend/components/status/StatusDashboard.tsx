'use client';

import { useState, useCallback, useEffect } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  Database,
  HardDrive,
  Zap,
  Shield,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useHealth, useHealthDeps, useCacheMetrics, usePoolStats } from '@/hooks/useApi';
import { getTraderErrorCopy } from '@/lib/api/trader-error-copy';
import { STATUS_PAGE_REFRESH_MS } from '@/lib/api/client';

interface ComponentStatus {
  [key: string]: string;
}

const STATUS_ICONS = {
  healthy: CheckCircle2,
  ok: CheckCircle2,
  unhealthy: XCircle,
  degraded: AlertTriangle,
  warning: AlertTriangle,
  not_configured: Clock,
  unknown: Clock,
};

const STATUS_COLORS = {
  healthy: 'text-emerald-600 dark:text-emerald-400',
  ok: 'text-emerald-600 dark:text-emerald-400',
  unhealthy: 'text-red-600 dark:text-red-400',
  degraded: 'text-amber-600 dark:text-amber-400',
  warning: 'text-amber-600 dark:text-amber-400',
  not_configured: 'text-muted-foreground',
  unknown: 'text-muted-foreground',
};

const STATUS_BG = {
  healthy: 'bg-emerald-500/10 border-emerald-500/20',
  ok: 'bg-emerald-500/10 border-emerald-500/20',
  unhealthy: 'bg-red-500/10 border-red-500/20',
  degraded: 'bg-amber-500/10 border-amber-500/20',
  warning: 'bg-amber-500/10 border-amber-500/20',
  not_configured: 'bg-muted/50 border-border',
  unknown: 'bg-muted/50 border-border',
};

// ---------------------------------------------------------------------------
// KPI Card — read-only metric display
// ---------------------------------------------------------------------------

interface KpiCardProps {
  label: string;
  value: string | number;
  description?: string;
  icon: React.ReactNode;
  loading?: boolean;
}

function KpiCard({ label, value, description, icon, loading }: KpiCardProps) {
  return (
    <Card className="bg-muted/30">
      <CardContent className="flex items-center gap-4 py-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
          {icon}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-muted-foreground">{label}</p>
          <p className="text-2xl font-bold truncate">
            {loading ? (
              <span className="inline-block h-6 w-16 animate-pulse rounded bg-muted" />
            ) : (
              value
            )}
          </p>
          {description && (
            <p className="text-xs text-muted-foreground truncate">{description}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function formatPercent(ratio: number): string {
  if (!Number.isFinite(ratio)) return '—';
  return `${(ratio * 100).toFixed(1)}%`;
}

function formatNumber(n: number): string {
  if (!Number.isFinite(n)) return '—';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

// ---------------------------------------------------------------------------
// Status helpers
// ---------------------------------------------------------------------------

function getStatusKey(status: string): keyof typeof STATUS_ICONS {
  const lowerStatus = status.toLowerCase();
  if (lowerStatus.includes('healthy')) return 'healthy';
  if (lowerStatus.includes('ok')) return 'ok';
  if (lowerStatus.includes('unhealthy')) return 'unhealthy';
  if (lowerStatus.includes('degraded')) return 'degraded';
  if (lowerStatus.includes('warning')) return 'warning';
  if (lowerStatus.includes('not_configured')) return 'not_configured';
  return 'unknown';
}

function ComponentStatusItem({
  name,
  status,
}: {
  name: string;
  status: string;
}) {
  const statusKey = getStatusKey(status);
  const Icon = STATUS_ICONS[statusKey];
  const colorClass = STATUS_COLORS[statusKey];
  const bgClass = STATUS_BG[statusKey];

  return (
    <div
      className={cn(
        'flex items-center justify-between p-4 rounded-lg border',
        bgClass
      )}
    >
      <div className="flex items-center gap-3">
        <Icon className={cn('h-5 w-5', colorClass)} />
        <div>
          <p className="font-medium capitalize">{name.replace(/_/g, ' ')}</p>
          <p className="text-sm text-muted-foreground capitalize">{status}</p>
        </div>
      </div>
      <Badge
        variant={
          statusKey === 'healthy' || statusKey === 'ok'
            ? 'default'
            : 'secondary'
        }
        className="capitalize"
      >
        {statusKey}
      </Badge>
    </div>
  );
}

export function StatusDashboard() {
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const healthIntervalMs = autoRefresh ? STATUS_PAGE_REFRESH_MS : undefined;

  const {
    data: healthData,
    loading: healthLoading,
    error: healthError,
    refresh: refreshHealth,
  } = useHealth(healthIntervalMs);

  const {
    data: depsData,
    loading: depsLoading,
    error: depsError,
    refresh: refreshDeps,
  } = useHealthDeps(healthIntervalMs);

  const {
    data: cacheData,
    loading: cacheLoading,
    refresh: refreshCache,
  } = useCacheMetrics(healthIntervalMs);

  const {
    data: poolData,
    loading: poolLoading,
    refresh: refreshPool,
  } = usePoolStats(healthIntervalMs);

  const loading = healthLoading || depsLoading;

  useEffect(() => {
    if (!healthLoading && !depsLoading && (healthData || depsData)) {
      setLastUpdated(new Date());
    }
  }, [healthLoading, depsLoading, healthData, depsData, cacheData, poolData]);

  const rawError = healthError ?? depsError ?? null;
  const errorMessage = rawError ? getTraderErrorCopy(rawError).headline : null;

  const handleRefresh = useCallback(() => {
    refreshHealth();
    refreshDeps();
    refreshCache();
    refreshPool();
  }, [refreshHealth, refreshDeps, refreshCache, refreshPool]);

  if (loading && !healthData) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw
          data-testid="icon"
          className="h-8 w-8 animate-spin text-muted-foreground"
        />
      </div>
    );
  }

  if (errorMessage && !healthData) {
    return (
      <Card className="border-red-500/20 bg-red-500/5">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-red-600 dark:text-red-400">
            <XCircle className="h-5 w-5" />
            Connection Error
          </CardTitle>
          <CardDescription>{errorMessage}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={handleRefresh} variant="outline">
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  const overallStatus = healthData?.status || 'unknown';
  const overallStatusKey = getStatusKey(overallStatus);
  const OverallIcon = STATUS_ICONS[overallStatusKey];

  return (
    <div className="space-y-6">
      <Card className={cn('border-2', STATUS_BG[overallStatusKey])}>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <OverallIcon
                className={cn('h-8 w-8', STATUS_COLORS[overallStatusKey])}
              />
              <div>
                <CardTitle className="text-2xl">
                  {overallStatusKey === 'healthy' || overallStatusKey === 'ok'
                    ? 'All Systems Operational'
                    : overallStatusKey === 'unhealthy'
                      ? 'Service Unhealthy'
                      : overallStatusKey === 'unknown'
                        ? 'Status Unavailable'
                        : 'Service Degraded'}
                </CardTitle>
                <CardDescription>
                  {lastUpdated && (
                    <>
                      Last updated: {lastUpdated.toLocaleTimeString()}
                      {' • '}
                      Version: {healthData?.version || 'unknown'}
                    </>
                  )}
                </CardDescription>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleRefresh}
                disabled={loading}
                aria-label="Refresh status"
              >
                <RefreshCw
                  className={cn('h-4 w-4', loading && 'animate-spin')}
                />
              </Button>
              <Button
                variant={autoRefresh ? 'default' : 'outline'}
                size="sm"
                onClick={() => setAutoRefresh(!autoRefresh)}
              >
                {autoRefresh ? 'Auto-refresh ON' : 'Auto-refresh OFF'}
              </Button>
            </div>
          </div>
        </CardHeader>
      </Card>

      {healthData && (
        <Card>
          <CardHeader>
            <CardTitle>Core Components</CardTitle>
            <CardDescription>
              Essential services required for StellarRoute operation
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {Object.keys(healthData.components ?? {}).length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No component details returned by the API.
              </p>
            ) : (
              Object.entries(healthData.components ?? {}).map(([name, status]) => (
                <ComponentStatusItem key={name} name={name} status={status} />
              ))
            )}
          </CardContent>
        </Card>
      )}

      {(cacheData || poolData || cacheLoading || poolLoading) && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="h-5 w-5" />
              Performance KPIs
            </CardTitle>
            <CardDescription>
              Read-only metrics from the StellarRoute API
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <KpiCard
                label="Cache Hit Ratio"
                value={cacheData ? formatPercent(cacheData.hit_ratio) : '—'}
                description={`${formatNumber(cacheData?.quote_hits ?? 0)} hits / ${formatNumber(cacheData?.quote_misses ?? 0)} misses`}
                icon={<HardDrive className="h-5 w-5" />}
                loading={cacheLoading}
              />
              <KpiCard
                label="Stale Rejections"
                value={cacheData ? formatNumber(cacheData.stale_quote_rejections) : '—'}
                description={`${formatNumber(cacheData?.stale_inputs_excluded ?? 0)} stale inputs excluded`}
                icon={<Shield className="h-5 w-5" />}
                loading={cacheLoading}
              />
              <KpiCard
                label="DB Connections (Primary)"
                value={poolData ? poolData.primary.size : '—'}
                description={`${poolData?.primary.in_use ?? 0} in use / ${poolData?.primary.idle ?? 0} idle`}
                icon={<Database className="h-5 w-5" />}
                loading={poolLoading}
              />
              <KpiCard
                label="DB Utilization (Primary)"
                value={poolData ? `${Math.round(poolData.primary.utilisation * 100)}%` : '—'}
                description={`Max ${poolData?.primary.max_connections ?? '—'} connections`}
                icon={<Zap className="h-5 w-5" />}
                loading={poolLoading}
              />
            </div>
          </CardContent>
        </Card>
      )}

      {depsData && (
        <Card>
          <CardHeader>
            <CardTitle>External Dependencies</CardTitle>
            <CardDescription>
              Third-party services and infrastructure components
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {Object.entries(depsData.components ?? {}).map(([name, status]) => (
              <ComponentStatusItem key={name} name={name} status={status} />
            ))}
          </CardContent>
        </Card>
      )}

      <Card className="bg-muted/30">
        <CardContent className="pt-6">
          <div className="text-sm text-muted-foreground space-y-2">
            <p>
              <strong>Status Indicators:</strong>
            </p>
            <ul className="list-disc list-inside space-y-1 ml-2">
              <li>
                <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                  Healthy/OK
                </span>{' '}
                - Service is fully operational
              </li>
              <li>
                <span className="text-amber-600 dark:text-amber-400 font-medium">
                  Warning
                </span>{' '}
                - Service is operational but experiencing elevated latency or
                lag
              </li>
              <li>
                <span className="text-red-600 dark:text-red-400 font-medium">
                  Unhealthy/Degraded
                </span>{' '}
                - Service is experiencing issues
              </li>
              <li>
                <span className="text-muted-foreground font-medium">
                  Not Configured
                </span>{' '}
                - Optional service not enabled
              </li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
