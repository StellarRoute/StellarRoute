import type { Story } from '@ladle/react';
import '@/app/globals.css';
import { PriceHistorySparkline } from './PriceHistorySparkline';
import type { PriceHistoryPoint } from '@/types';

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

const meta = { title: 'Shared/PriceHistorySparkline' };
export default meta;

/** Light wrapper giving the sparkline comfortable breathing room. */
function Frame({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-background text-foreground p-8">
      <div className="max-w-md mx-auto">{children}</div>
    </div>
  );
}

/** Generates a sine-wave price series starting at `base` (as a string). */
function makePriceHistory(n: number, base = 0.105, amplitude = 0.008): PriceHistoryPoint[] {
  const now = Date.now();
  return Array.from({ length: n }, (_, i) => ({
    timestamp: now - (n - 1 - i) * 60 * 60 * 1000, // hourly intervals, newest last
    price: (base + Math.sin(i / 3) * amplitude).toFixed(7),
  }));
}

/** 24 points — one per hour over the last 24 hours. */
const POINTS_24H = makePriceHistory(24, 0.1050000, 0.0030000);

/** Single data point (edge: trivial range). */
const POINTS_SINGLE: PriceHistoryPoint[] = [
  { timestamp: Date.now() - 60_000, price: '0.1050000' },
];

/** Flat price (edge: zero range collapses to y=50). */
const POINTS_FLAT: PriceHistoryPoint[] = Array.from({ length: 12 }, (_, i) => ({
  timestamp: Date.now() - (11 - i) * 60 * 60 * 1000,
  price: '1.0000000',
}));

/** Large round numbers to exercise the ≥1000 formatting branch. */
const POINTS_LARGE: PriceHistoryPoint[] = Array.from({ length: 10 }, (_, i) => ({
  timestamp: Date.now() - (9 - i) * 60 * 60 * 1000,
  price: (25000 + Math.sin(i) * 800).toFixed(7),
}));

// ---------------------------------------------------------------------------
// Stories
// ---------------------------------------------------------------------------

/** Pulsing skeleton shown while price history is being fetched. */
export const Loading: Story = () => (
  <Frame>
    <PriceHistorySparkline loading />
  </Frame>
);

/** Graceful empty state — no SDEX history available for this pair yet. */
export const Empty: Story = () => (
  <Frame>
    <PriceHistorySparkline points={[]} />
  </Frame>
);

/** Empty state with a custom title and label (e.g. for a different pair). */
export const EmptyCustomLabel: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={[]}
      title="7d price trend"
      emptyLabel="No 7-day price history available for this pair."
    />
  </Frame>
);

/** 24-hour data with multiple price points — the standard use-case. */
export const TwentyFourHour: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={POINTS_24H}
      title="24h price trend"
    />
  </Frame>
);

/**
 * Interactive hover story — move the pointer over the chart to reveal the
 * per-point tooltip showing timestamp and approximate price.
 */
export const HoverTooltip: Story = () => (
  <Frame>
    <p className="mb-4 text-xs text-muted-foreground">
      Hover or focus any point to reveal the price tooltip.
    </p>
    <PriceHistorySparkline
      points={POINTS_24H}
      title="24h price trend — hover any point"
    />
  </Frame>
);

/** Single data point — verifies the component handles a 1-point series. */
export const SinglePoint: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={POINTS_SINGLE}
      title="Minimal data"
    />
  </Frame>
);

/** Flat (zero-range) series — price collapses to the midline. */
export const FlatPrice: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={POINTS_FLAT}
      title="Stable price"
    />
  </Frame>
);

/** Large numbers (e.g. XLM/BTC satoshi price) — exercises integer formatting. */
export const LargeNumbers: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={POINTS_LARGE}
      title="BTC/USD 10-hour trend"
    />
  </Frame>
);

/** Custom className forwarding — adds a coloured ring around the container. */
export const CustomClassName: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={POINTS_24H}
      className="ring-2 ring-primary rounded-2xl p-2"
    />
  </Frame>
);

/**
 * Dark-background variant — most production placements use a dark card surface.
 * Compare against the default light wrapper above.
 */
export const DarkSurface: Story = () => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="max-w-md mx-auto space-y-6">
      <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
        Dark surface
      </h2>
      <PriceHistorySparkline points={POINTS_24H} />
      <PriceHistorySparkline points={[]} />
      <PriceHistorySparkline loading />
    </div>
  </div>
);
