import { cn } from '@/lib/utils';
import { TransactionStatusBadge } from '@/components/shared/TransactionStatusBadge';
import { VenueTypeBadge } from '@/components/shared/VenueTypeBadge';

const VENUE_TYPES: Array<'SDEX' | 'AMM' | 'Hybrid'> = ['SDEX', 'AMM', 'Hybrid'];
const TRANSACTION_STATUSES: Array<
  | 'pending'
  | 'submitted'
  | 'confirmed'
  | 'failed'
  | 'dropped'
> = ['pending', 'submitted', 'confirmed', 'failed', 'dropped'];

export function IconographyLegend({ className }: { className?: string }) {
  return (
    <section className={cn('rounded-3xl border border-border bg-muted/80 p-6 shadow-sm', className)}>
      <div className="flex flex-col gap-3">
        <div>
          <p className="text-sm uppercase tracking-[0.24em] text-muted-foreground font-semibold mb-2">
            Iconography System
          </p>
          <h2 className="text-xl font-semibold">Route and Transaction Icons</h2>
          <p className="text-sm text-muted-foreground mt-2">
            Consistent icons help users distinguish between venue types, hybrid routes, and transaction lifecycle states.
          </p>
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
          <div className="space-y-3">
            <h3 className="text-sm font-semibold">Venue Types</h3>
            <div className="flex flex-wrap gap-2">
              {VENUE_TYPES.map((type) => (
                <VenueTypeBadge key={type} type={type} size={20} />
              ))}
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground">
              <p>
                SDEX represents order book trades. AMM indicates liquidity pool swaps.
              </p>
              <p>
                Hybrid routes combine both venue types for optimal routing.
              </p>
            </div>
          </div>

          <div className="space-y-3">
            <h3 className="text-sm font-semibold">Transaction States</h3>
            <div className="flex flex-wrap gap-2">
              {TRANSACTION_STATUSES.map((status) => (
                <TransactionStatusBadge key={status} status={status} size={20} />
              ))}
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground">
              <p>
                Icons are sized for screen readability at 16/20/24px. Use light strokes for smaller badges and moderate stroke weight for larger route indicators.
              </p>
              <p>
                Asset icons fall back to stable uppercase initials when a valid image source is unavailable.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
