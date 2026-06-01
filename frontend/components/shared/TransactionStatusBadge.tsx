import { AlertTriangle, CheckCircle2, Clock, Loader2, XCircle, type LucideIcon } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { TransactionStatus } from '@/types/transaction';

const statusMeta: Record<
  TransactionStatus,
  { icon: LucideIcon; label: string; badgeClass: string }
> = {
  pending: {
    icon: Clock,
    label: 'Pending',
    badgeClass:
      'bg-secondary text-secondary-foreground border-transparent',
  },
  submitted: {
    icon: Loader2,
    label: 'Submitted',
    badgeClass:
      'bg-secondary text-secondary-foreground border-transparent',
  },
  confirmed: {
    icon: CheckCircle2,
    label: 'Confirmed',
    badgeClass:
      'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-200 border-transparent',
  },
  failed: {
    icon: XCircle,
    label: 'Failed',
    badgeClass: 'bg-destructive text-white border-transparent',
  },
  dropped: {
    icon: AlertTriangle,
    label: 'Dropped',
    badgeClass:
      'bg-muted text-foreground border-border/60 dark:bg-muted/80 dark:text-foreground',
  },
};

const sizeClasses: Record<16 | 20 | 24, string> = {
  16: 'w-4 h-4',
  20: 'w-5 h-5',
  24: 'w-6 h-6',
};

interface TransactionStatusBadgeProps {
  status: TransactionStatus;
  size?: 16 | 20 | 24;
  className?: string;
}

export function TransactionStatusBadge({
  status,
  size = 16,
  className,
}: TransactionStatusBadgeProps) {
  const meta = statusMeta[status];
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
      <Icon className={cn(sizeClasses[size], status === 'submitted' ? 'animate-spin' : '')} aria-hidden="true" />
      {meta.label}
    </Badge>
  );
}
