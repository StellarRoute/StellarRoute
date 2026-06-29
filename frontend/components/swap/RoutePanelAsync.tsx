'use client';

import dynamic from 'next/dynamic';
import { Skeleton } from '@/components/ui/skeleton';

function RouteDisplaySkeleton() {
  return (
    <div className="space-y-3" role="status" aria-label="Loading route panel">
      <Skeleton className="h-6 w-32" />
      <Skeleton className="h-24 w-full" />
      <Skeleton className="h-24 w-full" />
      <Skeleton className="h-24 w-full" />
    </div>
  );
}

const RouteDisplay = dynamic(
  () => import('./RouteDisplay').then((m) => m.RouteDisplay),
  {
    ssr: false,
    loading: () => <RouteDisplaySkeleton />,
  }
);

export default RouteDisplay;
