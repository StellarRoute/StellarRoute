'use client';

import React from 'react';
import { Card } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';

interface OrderbookSkeletonProps {
  rowCount?: number;
}

export function OrderbookSkeleton({ rowCount = 8 }: OrderbookSkeletonProps) {
  return (
    <div
      className="grid gap-4 md:grid-cols-2"
      aria-busy="true"
      aria-label="Loading orderbook"
      data-testid="orderbook-skeleton"
    >
      {/* Bids Card Skeleton */}
      <Card className="p-4 space-y-3">
        <div className="flex items-center justify-between">
          <Skeleton className="h-5 w-24" />
        </div>
        <div className="space-y-1 text-sm">
          <div className="grid grid-cols-3 text-xs pb-2 border-b">
            <Skeleton className="h-3 w-12" />
            <Skeleton className="h-3 w-14" />
            <Skeleton className="h-3 w-12" />
          </div>
          <div className="space-y-2 pt-1" data-testid="bids-skeleton-rows">
            {Array.from({ length: rowCount }).map((_, index) => (
              <div
                key={`bid-skel-${index}`}
                className="grid grid-cols-3 py-1.5 px-2 items-center"
                style={{ height: '36px' }}
                data-testid="orderbook-skeleton-row"
              >
                <Skeleton className="h-4 w-16 bg-emerald-500/10" />
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-4 w-16" />
              </div>
            ))}
          </div>
        </div>
      </Card>

      {/* Asks Card Skeleton */}
      <Card className="p-4 space-y-3">
        <div className="flex items-center justify-between">
          <Skeleton className="h-5 w-24" />
        </div>
        <div className="space-y-1 text-sm">
          <div className="grid grid-cols-3 text-xs pb-2 border-b">
            <Skeleton className="h-3 w-12" />
            <Skeleton className="h-3 w-14" />
            <Skeleton className="h-3 w-12" />
          </div>
          <div className="space-y-2 pt-1" data-testid="asks-skeleton-rows">
            {Array.from({ length: rowCount }).map((_, index) => (
              <div
                key={`ask-skel-${index}`}
                className="grid grid-cols-3 py-1.5 px-2 items-center"
                style={{ height: '36px' }}
                data-testid="orderbook-skeleton-row"
              >
                <Skeleton className="h-4 w-16 bg-red-500/10" />
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-4 w-16" />
              </div>
            ))}
          </div>
        </div>
      </Card>
    </div>
  );
}
