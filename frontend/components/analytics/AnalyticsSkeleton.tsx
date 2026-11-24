"use client";

import { Skeleton } from "@/components/ui/skeleton";

/**
 * Skeleton placeholder rendered while analytics metrics are loading.
 * Mirrors the visual structure of AnalyticsDashboard so the layout doesn't
 * shift when real data arrives.
 */
export function AnalyticsSkeleton() {
  return (
    <div
      className="w-full px-4 py-8 sm:px-6 lg:px-8 space-y-6"
      aria-label="Loading analytics"
      aria-busy="true"
      data-testid="analytics-skeleton"
    >
      {/* Page header */}
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-2">
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-4 w-72" />
        </div>
        <Skeleton className="h-9 w-24 rounded-md" />
      </div>

      {/* Metrics cards grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {/* Quote cache card */}
        <div className="rounded-lg border p-6 space-y-4">
          <div className="space-y-2">
            <Skeleton className="h-4 w-28" />
            <Skeleton className="h-3 w-44" />
          </div>
          <Skeleton className="h-10 w-20" />
          <div className="grid grid-cols-2 gap-3">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
          </div>
        </div>

        {/* Primary pool card */}
        <div className="rounded-lg border p-6 space-y-4">
          <div className="space-y-2">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-3 w-40" />
          </div>
          <div className="flex items-end justify-between gap-4">
            <div className="space-y-1">
              <Skeleton className="h-10 w-20" />
              <Skeleton className="h-3 w-16" />
            </div>
            <Skeleton className="h-2 flex-1 max-w-[160px] rounded-full" />
          </div>
        </div>

        {/* Replica pool card */}
        <div className="rounded-lg border p-6 space-y-4">
          <div className="space-y-2">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-3 w-36" />
          </div>
          <div className="flex items-end justify-between gap-4">
            <div className="space-y-1">
              <Skeleton className="h-10 w-20" />
              <Skeleton className="h-3 w-16" />
            </div>
            <Skeleton className="h-2 flex-1 max-w-[160px] rounded-full" />
          </div>
        </div>
      </div>
    </div>
  );
}
