import { ArrowRightLeft, Droplet, Layers, type LucideIcon } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export type VenueType = 'SDEX' | 'AMM' | 'Hybrid';

const venueMeta: Record<VenueType, { icon: LucideIcon; badgeClass: string }> = {
  SDEX: {
    icon: Layers,
    badgeClass:
      'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-200 border-transparent',
  },
  AMM: {
    icon: Droplet,
    badgeClass:
      'bg-purple-100 text-purple-700 dark:bg-purple-950 dark:text-purple-200 border-transparent',
  },
  Hybrid: {
    icon: ArrowRightLeft,
    badgeClass:
      'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-200 border-transparent',
  },
};

const sizeClasses: Record<16 | 20 | 24, string> = {
  16: 'w-4 h-4',
  20: 'w-5 h-5',
  24: 'w-6 h-6',
};

interface VenueTypeBadgeProps {
  type: VenueType;
  size?: 16 | 20 | 24;
  className?: string;
}

export function VenueTypeBadge({ type, size = 16, className }: VenueTypeBadgeProps) {
  const meta = venueMeta[type];
  const Icon = meta.icon;

  return (
    <Badge
      variant="secondary"
      className={cn(
        'gap-2 px-2 py-1 text-[10px] uppercase font-semibold tracking-wider',
        size === 24 ? 'text-[11px]' : 'text-[10px]',
        meta.badgeClass,
        className
      )}
    >
      <Icon className={cn(sizeClasses[size])} aria-hidden="true" />
      {type}
    </Badge>
  );
}
