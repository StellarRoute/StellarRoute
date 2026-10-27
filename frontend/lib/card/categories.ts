/**
 * CARD-53: Merchant category mapping (MCC → spending buckets)
 * and percentage breakdown helpers for settled POS debits only.
 */

export type SpendCategory =
  | 'Dining'
  | 'Travel'
  | 'Groceries'
  | 'Tech/SaaS'
  | 'General';

export const CATEGORY_COLORS: Record<SpendCategory, string> = {
  Dining: '#f97316', // orange
  Travel: '#3b82f6', // blue
  Groceries: '#22c55e', // green
  'Tech/SaaS': '#a855f7', // purple
  General: '#94a3b8', // slate
};

/** Common MCC ranges → friendly category */
const MCC_RULES: Array<{ start: number; end: number; category: SpendCategory }> = [
  { start: 5812, end: 5814, category: 'Dining' }, // restaurants / fast food
  { start: 5811, end: 5811, category: 'Dining' },
  { start: 3000, end: 3299, category: 'Travel' }, // airlines
  { start: 3500, end: 3999, category: 'Travel' }, // hotels
  { start: 4111, end: 4171, category: 'Travel' }, // transit
  { start: 4411, end: 4789, category: 'Travel' },
  { start: 5411, end: 5499, category: 'Groceries' },
  { start: 5300, end: 5399, category: 'Groceries' },
  { start: 5734, end: 5734, category: 'Tech/SaaS' },
  { start: 5815, end: 5818, category: 'Tech/SaaS' },
  { start: 7372, end: 7372, category: 'Tech/SaaS' },
];

export function categoryFromMcc(mcc: string | number | undefined | null): SpendCategory {
  if (mcc === undefined || mcc === null || mcc === '') return 'General';
  const code = typeof mcc === 'number' ? mcc : parseInt(String(mcc), 10);
  if (Number.isNaN(code)) return 'General';

  for (const rule of MCC_RULES) {
    if (code >= rule.start && code <= rule.end) return rule.category;
  }
  return 'General';
}

export interface DebitLike {
  type?: string;
  /** only settled pos debits count */
  status?: string;
  usdcAmount: number; // usually negative for debit
  mcc?: string | number;
}

export interface CategoryBucket {
  category: SpendCategory;
  totalUsdc: number;
  count: number;
  percent: number; // 0–100
  color: string;
}

/**
 * Build spend breakdown from activity-like rows.
 * - Includes only pos_debit (and optional settled filter)
 * - Excludes cctp_deposit / positive top-ups
 * - Percentages sum ~100 (last bucket adjusted for rounding)
 */
export function buildCategoryBreakdown(items: DebitLike[]): CategoryBucket[] {
  const settledDebits = items.filter((row) => {
    if (row.type && row.type !== 'pos_debit') return false;
    if (row.usdcAmount >= 0) return false; // exclude deposits / zero
    if (row.status && row.status !== 'settled' && row.status !== 'captured') {
      // if status present, only settled-like; if absent, still allow
      return row.status === undefined;
    }
    return true;
  });

  const totals = new Map<SpendCategory, { totalUsdc: number; count: number }>();

  for (const row of settledDebits) {
    const cat = categoryFromMcc(row.mcc);
    const abs = Math.abs(row.usdcAmount);
    const prev = totals.get(cat) ?? { totalUsdc: 0, count: 0 };
    prev.totalUsdc += abs;
    prev.count += 1;
    totals.set(cat, prev);
  }

  const grand = [...totals.values()].reduce((s, v) => s + v.totalUsdc, 0);
  if (grand <= 0) return [];

  const categories = [...totals.keys()];
  const buckets: CategoryBucket[] = categories.map((category) => {
    const { totalUsdc, count } = totals.get(category)!;
    const raw = (totalUsdc / grand) * 100;
    return {
      category,
      totalUsdc: Math.round(totalUsdc * 100) / 100,
      count,
      percent: Math.round(raw * 10) / 10, // 1 decimal
      color: CATEGORY_COLORS[category],
    };
  });

  // Fix rounding so percents sum to 100
  if (buckets.length > 0) {
    const sum = buckets.reduce((s, b) => s + b.percent, 0);
    const diff = Math.round((100 - sum) * 10) / 10;
    buckets[buckets.length - 1].percent =
      Math.round((buckets[buckets.length - 1].percent + diff) * 10) / 10;
  }

  // Largest first for nicer bar
  buckets.sort((a, b) => b.totalUsdc - a.totalUsdc);
  return buckets;
}
