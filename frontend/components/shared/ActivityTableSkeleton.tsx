"use client";

import { Skeleton } from "@/components/ui/skeleton";

/**
 * Skeleton loader for transaction activity table
 * Renders 5 skeleton rows matching the table structure to prevent layout shift
 */
export function ActivityTableSkeleton() {
  return (
    <div className="w-full">
      {/* Table header skeleton */}
      <div className="border-b flex bg-muted/50 sticky top-0">
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-16" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-24" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-20" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-16" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-20" />
        </div>
        <div className="flex-1 p-4">
          <Skeleton className="h-4 w-20" />
        </div>
      </div>

      {/* 5 skeleton rows */}
      {Array.from({ length: 5 }).map((_, rowIndex) => (
        <div key={rowIndex} className="border-b flex hover:bg-muted/50 transition-colors">
          {/* Date column */}
          <div className="flex-1 p-4">
            <div className="space-y-2">
              <Skeleton className="h-4 w-20" />
              <Skeleton className="h-3 w-24" />
            </div>
          </div>

          {/* Swap column */}
          <div className="flex-1 p-4">
            <div className="flex items-center gap-3">
              <Skeleton className="h-8 w-16" />
              <Skeleton className="h-4 w-4" />
              <Skeleton className="h-8 w-16" />
            </div>
          </div>

          {/* Rate column */}
          <div className="flex-1 p-4">
            <Skeleton className="h-4 w-28" />
          </div>

          {/* Status column */}
          <div className="flex-1 p-4">
            <Skeleton className="h-6 w-20" />
          </div>

          {/* Amount column */}
          <div className="flex-1 p-4">
            <Skeleton className="h-4 w-24" />
          </div>

          {/* Explorer column */}
          <div className="flex-1 p-4">
            <Skeleton className="h-8 w-8" />
          </div>
        </div>
      ))}
    </div>
  );
}
