import type { Story } from '@ladle/react';
import { useEffect } from 'react';

import { AnalyticsDashboard } from './AnalyticsDashboard';
import type { CacheMetricsResponse, PoolStatsResponse } from '@/types';

const cacheMetricsFixture: CacheMetricsResponse = {
  quote_hits: 1427,
  quote_misses: 273,
  hit_ratio: 0.84,
  stale_quote_rejections: 11,
  stale_inputs_excluded: 4,
};

const poolStatsFixture: PoolStatsResponse = {
  primary: {
    max_connections: 30,
    size: 22,
    idle: 8,
    in_use: 14,
    utilisation: 0.73,
  },
  replica: {
    max_connections: 20,
    size: 16,
    idle: 7,
    in_use: 9,
    utilisation: 0.62,
  },
};

function mockMetricsResponses({
  cache,
  pool,
  pending = false,
}: {
  cache?: CacheMetricsResponse;
  pool?: PoolStatsResponse;
  pending?: boolean;
}) {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = (async (
    input: RequestInfo | URL,
    init?: RequestInit,
  ) => {
    const url = String(input);

    if (url.includes('/metrics/cache')) {
      if (pending) {
        return await new Promise<Response>((resolve) => {
          setTimeout(() => {
            resolve(
              new Response(JSON.stringify(cacheMetricsFixture), {
                status: 200,
                headers: { 'Content-Type': 'application/json' },
              }),
            );
          }, 1200);
        });
      }

      return new Response(JSON.stringify(cache ?? cacheMetricsFixture), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    if (url.includes('/metrics/pool')) {
      if (pending) {
        return await new Promise<Response>((resolve) => {
          setTimeout(() => {
            resolve(
              new Response(JSON.stringify(poolStatsFixture), {
                status: 200,
                headers: { 'Content-Type': 'application/json' },
              }),
            );
          }, 1200);
        });
      }

      return new Response(JSON.stringify(pool ?? poolStatsFixture), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    if (init?.signal?.aborted) {
      throw init.signal.reason ?? new Error('Request aborted');
    }

    return new Response(JSON.stringify({}), {
      status: 404,
      headers: { 'Content-Type': 'application/json' },
    });
  }) as typeof fetch;

  return () => {
    globalThis.fetch = originalFetch;
  };
}

function StoryHarness({
  cache,
  pool,
  pending = false,
}: {
  cache?: CacheMetricsResponse;
  pool?: PoolStatsResponse;
  pending?: boolean;
}) {
  useEffect(() => {
    const restore = mockMetricsResponses({ cache, pool, pending });
    return restore;
  }, [cache, pool, pending]);

  return <AnalyticsDashboard />;
}

export const Loading: Story = () => <StoryHarness pending />;

export const Populated: Story = () => (
  <StoryHarness cache={cacheMetricsFixture} pool={poolStatsFixture} />
);

export const Data: Story = Populated;
