'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';

import { TradeActivityTable } from '@/components/shared/TradeActivityTable';
import { ViewState } from '@/components/shared/ViewState';
import { Button } from '@/components/ui/button';
import { useStellarRouteClient } from '@/hooks/useStellarRouteClient';
import type { TradeRecord } from '@/types/trade';

/** How many events to request per fetch page. */
const HISTORY_PAGE_SIZE = 500;

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
 * Owns the fetch for the page so the empty state can carry an illustration.
 * Passes `initialData` to `TradeActivityTable`, which paginates 10 rows at a
 * time client-side. A "Load more" button re-fetches with an increased limit
 * when the previous response was full (indicating the API has more records).
 */
export function HistoryPageClient({ address }: { address?: string }) {
  const client = useStellarRouteClient();
  const [records, setRecords] = useState<TradeRecord[] | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [fetchLimit, setFetchLimit] = useState(HISTORY_PAGE_SIZE);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  /** True when the last response was exactly `fetchLimit` rows — there may be more. */
  const [mayHaveMore, setMayHaveMore] = useState(false);

  const fetchRecords = useCallback(
    (limit: number, append: boolean) => {
      if (!address) {
        setRecords([]);
        return () => {};
      }

      let active = true;
      if (!append) {
        setRecords(null);
        setError(null);
      }

      client
        .getSwapActivity({ limit })
        .then((res) => {
          if (!active) return;

          const mapped: TradeRecord[] = res.swaps
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
            }));

          setRecords(mapped);
          // If we got exactly `limit` rows the API may have more beyond this window.
          setMayHaveMore(res.swaps.length >= limit);
          setIsLoadingMore(false);
        })
        .catch((err: unknown) => {
          if (!active) return;
          setError(err instanceof Error ? err : new Error(String(err)));
          if (!append) setRecords([]);
          setIsLoadingMore(false);
        });

      return () => {
        active = false;
      };
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [address, client],
  );

  useEffect(() => {
    const cleanup = fetchRecords(fetchLimit, false);
    return cleanup;
  }, [address, fetchLimit, fetchRecords]);

  const handleLoadMore = () => {
    const nextLimit = fetchLimit + HISTORY_PAGE_SIZE;
    setFetchLimit(nextLimit);
    setIsLoadingMore(true);
  };

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

  return (
    <div className="space-y-4">
      <TradeActivityTable address={address} initialData={records} />
      {mayHaveMore && (
        <div className="flex justify-center pt-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleLoadMore}
            disabled={isLoadingMore}
            data-testid="history-load-more"
          >
            {isLoadingMore ? 'Loading…' : 'Load more'}
          </Button>
        </div>
      )}
    </div>
  );
}

export default HistoryPageClient;
