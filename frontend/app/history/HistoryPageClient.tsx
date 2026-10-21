'use client';

import { useEffect, useMemo, useState } from 'react';

import { TradeActivityTable } from '@/components/shared/TradeActivityTable';
import { ViewState } from '@/components/shared/ViewState';
import { Button } from '@/components/ui/button';
import { useStellarRouteClient } from '@/hooks/useStellarRouteClient';
import type { TradeRecord } from '@/types/trade';

/**
 * Decorative empty-state artwork for this page (issue #1263).
 *
 * `aria-hidden` with an empty `alt`: the surrounding `ViewState` title and
 * description already carry the meaning, per docs/design/empty-states-spec.md.
 */
function EmptyHistoryIllustration() {
  return (
    // eslint-disable-next-line @next/next/no-img-element -- static SVG from /public, no optimisation needed
    <img
      src="/illustrations/empty-history.svg"
      alt=""
      aria-hidden="true"
      width={120}
      height={96}
      className="h-24 w-auto opacity-90"
    />
  );
}

/**
 * Client shell for /history.
 *
 * Owns the single fetch for the page so the empty state can carry an
 * illustration: `TradeActivityTable` renders its own bare "No trade activity
 * found." text when it fetches for itself, and it is outside this issue's
 * scope. Passing `initialData` hands it the already-fetched rows, so the table
 * does no second request and keeps its own sorting and pagination over the
 * full result set.
 */
export function HistoryPageClient({ address }: { address?: string }) {
  const client = useStellarRouteClient();
  const [records, setRecords] = useState<TradeRecord[] | null>(null);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!address) {
      setRecords([]);
      return;
    }

    let active = true;
    setRecords(null);
    setError(null);

    client
      .getSwapActivity({ limit: 100 })
      .then((res) => {
        if (!active) return;
        // Same shape the table's own hook produces, so rendering is unchanged.
        setRecords(
          res.swaps
            .filter(
              (swap) => swap.sender.toLowerCase() === address.toLowerCase()
            )
            .map((swap) => ({
              id: swap.event_id,
              txHash: swap.paging_token,
              timestamp: swap.ledger_closed_at
                ? new Date(swap.ledger_closed_at)
                : new Date(),
              action: 'SWAP',
              amount: swap.amount_in,
              asset: `${swap.source_asset || 'Unknown'} → ${swap.destination_asset || 'Unknown'}`,
            }))
        );
      })
      .catch((err: unknown) => {
        if (!active) return;
        setError(err instanceof Error ? err : new Error(String(err)));
        setRecords([]);
      });

    return () => {
      active = false;
    };
  }, [address, client]);

  const isLoading = records === null;
  const isEmpty = useMemo(
    () => records !== null && records.length === 0 && !error,
    [records, error]
  );

  if (isLoading) {
    return (
      <ViewState
        variant="loading"
        title="Loading transactions"
        description="Fetching your swap history..."
      />
    );
  }

  if (error) {
    return (
      <ViewState
        variant="error"
        title="Could not load history"
        description="The transaction service is temporarily unavailable. Please try again."
      />
    );
  }

  if (isEmpty) {
    return (
      <ViewState
        variant="empty"
        title="No transactions yet"
        description="You haven't made any swaps. Head to the Swap page to get started."
        illustration={<EmptyHistoryIllustration />}
        action={
          <Button asChild variant="default">
            <a href="/swap" aria-label="Go to the swap page to make your first swap">
              Make your first swap
            </a>
          </Button>
        }
      />
    );
  }

  return <TradeActivityTable address={address} initialData={records} />;
}

export default HistoryPageClient;
