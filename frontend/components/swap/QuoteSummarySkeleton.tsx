"use client";

import { Skeleton } from "@/components/ui/skeleton";

/**
 * Skeleton loader for QuoteSummary component
 * Matches the layout and spacing of QuoteSummary to prevent layout shift
 */
export function QuoteSummarySkeleton() {
  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      {/* Rate row */}
      <div className="flex justify-between items-center gap-2">
        <span className="text-sm text-muted-foreground">Rate</span>
        <Skeleton className="h-5 w-32" />
      </div>

      {/* Network Fee row */}
      <div className="flex justify-between items-center gap-2">
        <span className="text-sm text-muted-foreground">Network Fee</span>
        <Skeleton className="h-5 w-24" />
      </div>

      {/* Price Impact row */}
      <div className="flex justify-between items-center gap-2">
        <span className="text-sm text-muted-foreground">Price Impact</span>
        <Skeleton className="h-5 w-20" />
      </div>
    </div>
  );
}
