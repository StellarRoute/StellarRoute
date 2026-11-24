'use client';

import {
  buildCategoryBreakdown,
  type DebitLike,
  type CategoryBucket,
} from '../../lib/card/categories';

export interface CardCategoryBreakdownProps {
  items: DebitLike[];
}

export function CardCategoryBreakdown({ items }: CardCategoryBreakdownProps) {
  const buckets = buildCategoryBreakdown(items);

  if (buckets.length === 0) {
    return (
      <div data-testid="category-empty" className="text-sm text-slate-500">
        No settled card spending yet
      </div>
    );
  }

  return (
    <div data-testid="category-breakdown" className="space-y-3">
      <div
        data-testid="category-bar"
        className="flex h-3 w-full overflow-hidden rounded-full bg-slate-100"
        role="img"
        aria-label="Spending by category"
      >
        {buckets.map((b) => (
          <CategorySegment key={b.category} bucket={b} />
        ))}
      </div>

      <ul data-testid="category-legend" className="space-y-1">
        {buckets.map((b) => (
          <li
            key={b.category}
            data-testid={`legend-${b.category}`}
            className="flex items-center justify-between text-sm"
          >
            <span className="flex items-center gap-2">
              <span
                className="inline-block h-2.5 w-2.5 rounded-full"
                style={{ backgroundColor: b.color }}
                data-testid={`badge-${b.category}`}
              />
              {b.category}
            </span>
            <span className="text-slate-600">
              {b.totalUsdc.toFixed(2)} USDC · {b.percent}%
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function CategorySegment({ bucket }: { bucket: CategoryBucket }) {
  const title = `${bucket.category}: ${bucket.totalUsdc.toFixed(2)} USDC (${bucket.count} tx) · ${bucket.percent}%`;
  return (
    <div
      data-testid={`segment-${bucket.category}`}
      title={title}
      aria-label={title}
      tabIndex={0}
      style={{
        width: `${bucket.percent}%`,
        backgroundColor: bucket.color,
      }}
      className="h-full min-w-0 outline-none focus:ring-2 focus:ring-offset-1"
    />
  );
}