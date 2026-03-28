"use client";

import { Skeleton } from "@/components/ui/skeleton";

/**
 * Skeleton loader for RouteDisplay component
 * Matches the layout and spacing of RouteDisplay to prevent layout shift
 */
export function RouteDisplaySkeleton() {
  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-4 transition-all duration-200">
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Skeleton className="h-5 w-20" />
        </div>
        <div className="flex items-center gap-2">
          <Skeleton className="h-8 w-16" />
          <Skeleton className="h-8 w-16" />
        </div>
      </div>

      {/* Route path row */}
      <div className="flex items-center bg-muted/50 rounded-lg p-3 gap-2 justify-between">
        <Skeleton className="h-8 w-16" />
        <Skeleton className="h-4 w-12" />
        <Skeleton className="h-8 w-20" />
        <Skeleton className="h-4 w-12" />
        <Skeleton className="h-8 w-20" />
      </div>

      {/* Alternative routes section */}
      <div className="pt-3 border-t border-border/50">
        <Skeleton className="h-4 w-32 mb-2" />
        <Skeleton className="h-8 w-full" />
      </div>
    </div>
  );
}
