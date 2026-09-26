import type { Story } from '@ladle/react';
import type { ReactNode } from 'react';
import '@/app/globals.css';
import { PriceHistorySparkline } from './PriceHistorySparkline';
import type { PriceHistoryPoint } from '@/types';

const NOW = Date.UTC(2025, 0, 1, 12, 0, 0);
const HOUR = 60 * 60 * 1000;

/** 24 hourly mid-market points with a gentle upward drift. */
const DAY_POINTS: PriceHistoryPoint[] = Array.from({ length: 24 }, (_, i) => ({
  timestamp: NOW - (23 - i) * HOUR,
  price: (0.1125 + Math.sin(i / 3) * 0.0025 + i * 0.00015).toFixed(6),
}));

const Frame = ({ children }: { children: ReactNode }) => (
  <div className="dark min-h-screen bg-background text-foreground p-8">
    <div className="mx-auto max-w-md">{children}</div>
  </div>
);

/** Loading skeleton while 24h history is being fetched. */
export const Loading: Story = () => (
  <Frame>
    <PriceHistorySparkline loading />
  </Frame>
);
Loading.storyName = 'Price History — Loading';

/** Empty state — no history points available for the pair. */
export const Empty: Story = () => (
  <Frame>
    <PriceHistorySparkline points={[]} />
  </Frame>
);
Empty.storyName = 'Price History — Empty';

/** Populated 24h series. */
export const TwentyFourHours: Story = () => (
  <Frame>
    <PriceHistorySparkline points={DAY_POINTS} />
  </Frame>
);
TwentyFourHours.storyName = 'Price History — 24h Data';

/** Same 24h series — hover the chart to reveal the point tooltip. */
export const HoverTooltip: Story = () => (
  <Frame>
    <PriceHistorySparkline
      points={DAY_POINTS}
      title="24h price trend (hover for detail)"
    />
  </Frame>
);
HoverTooltip.storyName = 'Price History — Hover Tooltip';
