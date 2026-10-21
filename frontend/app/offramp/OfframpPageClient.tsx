'use client';

import { useSearchParams } from 'next/navigation';
import { OfframpDashboard } from '@/components/offramp/OfframpDashboard';
import { DEFAULT_OFFRAMP_SOURCE_ID, findOfframpSource } from '@/lib/offramp';

export function OfframpPageClient() {
  const searchParams = useSearchParams();
  const amount = searchParams.get('amount');
  const source = searchParams.get('source');

  const hasPrefill = Boolean(amount && source && findOfframpSource(source));

  return (
    <OfframpDashboard
      prefill={
        hasPrefill
          ? {
              amount,
              sourceId: source ?? DEFAULT_OFFRAMP_SOURCE_ID,
            }
          : undefined
      }
    />
  );
}
