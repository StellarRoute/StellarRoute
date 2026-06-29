"use client";

import { AnalyticsDashboard } from "@/components/analytics/AnalyticsDashboard";
import { ViewState } from "@/components/shared/ViewState";
import { useFeatureFlag } from "@/hooks/useFeatureFlag";

/**
 * Client wrapper for /analytics.
 *
 * Access policy:
 * - Public read-only dashboard (no wallet or auth required).
 * - Live metrics are sourced from GET /metrics/cache and GET /metrics/pool.
 * - Gated by the `analytics` feature flag (`NEXT_PUBLIC_FEATURE_ANALYTICS`).
 *   When disabled, the route remains reachable but shows a placeholder instead
 *   of live API metrics.
 */
export function AnalyticsPageClient() {
  const { enabled: analyticsEnabled } = useFeatureFlag("analytics");

  if (!analyticsEnabled) {
    return (
      <div className="w-full px-4 py-8 sm:px-6 lg:px-8 space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Analytics</h1>
          <p className="text-muted-foreground">
            Trading and platform analytics for StellarRoute.
          </p>
        </div>
        <ViewState
          variant="empty"
          title="Analytics preview disabled"
          description="Enable the analytics feature flag (NEXT_PUBLIC_FEATURE_ANALYTICS=true) to load live platform metrics."
        />
      </div>
    );
  }

  return <AnalyticsDashboard />;
}
