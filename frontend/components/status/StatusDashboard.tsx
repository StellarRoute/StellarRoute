'use client';

import { useEffect, useState, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { RefreshCw, CheckCircle2, XCircle, AlertTriangle, Clock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { API_BASE_URL } from '@/lib/constants';

interface ComponentStatus {
  [key: string]: string;
}

interface HealthData {
  status: string;
  timestamp: string;
  version: string;
  components: ComponentStatus;
}

interface DependencyHealthData {
  status: string;
  timestamp: string;
  components: ComponentStatus;
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

function ComponentStatusItem({ name, status }: { name: string; status: string }) {
  const statusKey = getStatusKey(status);
  const Icon = STATUS_ICONS[statusKey];
  const colorClass = STATUS_COLORS[statusKey];
  const bgClass = STATUS_BG[statusKey];

  return (
    <div className={cn('flex items-center justify-between p-4 rounded-lg border', bgClass)}>
      <div className="flex items-center gap-3">
        <Icon className={cn('h-5 w-5', colorClass)} />
        <div>
          <p className="font-medium capitalize">
            {name.replace(/_/g, ' ')}
          </p>
          <p className="text-sm text-muted-foreground capitalize">{status}</p>
        </div>
      </div>
      <Badge
        variant={statusKey === 'healthy' || statusKey === 'ok' ? 'default' : 'secondary'}
        className="capitalize"
      >
        {statusKey}
      </Badge>
    </div>
  );
}

export function StatusDashboard() {
  const [healthData, setHealthData] = useState<HealthData | null>(null);
  const [depsData, setDepsData] = useState<DependencyHealthData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchStatus = useCallback(async () => {
    try {
      setError(null);
      
      // Fetch basic health
      const healthRes = await fetch(`${API_BASE_URL.replace('/api/v1', '')}/health`);
      const healthJson = await healthRes.json();
      setHealthData(healthJson.data);

      // Fetch dependency health
      const depsRes = await fetch(`${API_BASE_URL.replace('/api/v1', '')}/health/deps`);
      const depsJson = await depsRes.json();
      setDepsData(depsJson.data);

      setLastUpdated(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch status');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      fetchStatus();
    }, 30000); // Refresh every 30 seconds

    return () => clearInterval(interval);
  }, [autoRefresh, fetchStatus]);

  const handleRefresh = () => {
    setLoading(true);
    fetchStatus();
  };

  if (loading && !healthData) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error && !healthData) {
    return (
      <Card className="border-red-500/20 bg-red-500/5">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-red-600 dark:text-red-400">
            <XCircle className="h-5 w-5" />
            Connection Error
          </CardTitle>
          <CardDescription>{error}</CardDescription>
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
      {/* Overall Status Card */}
      <Card className={cn('border-2', STATUS_BG[overallStatusKey])}>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <OverallIcon className={cn('h-8 w-8', STATUS_COLORS[overallStatusKey])} />
              <div>
                <CardTitle className="text-2xl">
                  {overallStatusKey === 'healthy' || overallStatusKey === 'ok'
                    ? 'All Systems Operational'
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
              >
                <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />
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

      {/* Core Components */}
      {healthData && (
        <Card>
          <CardHeader>
            <CardTitle>Core Components</CardTitle>
            <CardDescription>
              Essential services required for StellarRoute operation
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {Object.entries(healthData.components).map(([name, status]) => (
              <ComponentStatusItem key={name} name={name} status={status} />
            ))}
          </CardContent>
        </Card>
      )}

      {/* Dependencies */}
      {depsData && (
        <Card>
          <CardHeader>
            <CardTitle>External Dependencies</CardTitle>
            <CardDescription>
              Third-party services and infrastructure components
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {Object.entries(depsData.components).map(([name, status]) => (
              <ComponentStatusItem key={name} name={name} status={status} />
            ))}
          </CardContent>
        </Card>
      )}

      {/* Info Footer */}
      <Card className="bg-muted/30">
        <CardContent className="pt-6">
          <div className="text-sm text-muted-foreground space-y-2">
            <p>
              <strong>Status Indicators:</strong>
            </p>
            <ul className="list-disc list-inside space-y-1 ml-2">
              <li><span className="text-emerald-600 dark:text-emerald-400 font-medium">Healthy/OK</span> - Service is fully operational</li>
              <li><span className="text-amber-600 dark:text-amber-400 font-medium">Warning</span> - Service is operational but experiencing elevated latency or lag</li>
              <li><span className="text-red-600 dark:text-red-400 font-medium">Unhealthy/Degraded</span> - Service is experiencing issues</li>
              <li><span className="text-muted-foreground font-medium">Not Configured</span> - Optional service not enabled</li>
            </ul>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
