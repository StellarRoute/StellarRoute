import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface VenueBadgeLegendProps {
  className?: string;
}

export function VenueBadgeLegend({ className }: VenueBadgeLegendProps) {
  const legendItems = [
    {
      label: 'Optimal',
      badgeClass: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20 border',
      description: 'The most efficient route chosen by the aggregator based on price, gas, and price impact.',
    },
    {
      label: 'SDEX',
      badgeClass: 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200 border-transparent',
      description: 'Stellar Decentralized Exchange (orderbook-based liquidity).',
    },
    {
      label: 'AMM',
      badgeClass: 'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200 border-transparent',
      description: 'Automated Market Maker (liquidity pool swaps).',
    },
  ];

  return (
    <div className={cn("space-y-3.5", className)} role="region" aria-label="Venue badge legend">
      <div className="border-b border-border/50 pb-2">
        <h5 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Venue & Route Badges
        </h5>
        <p className="text-[11px] text-muted-foreground mt-0.5">
          Understanding the route markers and order venues.
        </p>
      </div>
      <div className="space-y-3">
        {legendItems.map((item) => (
          <div key={item.label} className="flex items-start gap-3 text-left">
            <Badge
              variant="secondary"
              className={cn("text-[10px] uppercase font-bold py-0.5 px-2 tracking-wider", item.badgeClass)}
            >
              {item.label}
            </Badge>
            <div className="flex-1 space-y-0.5">
              <span className="text-xs font-medium block leading-none">{item.label}</span>
              <p className="text-[11px] text-muted-foreground leading-normal mt-1">
                {item.description}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
