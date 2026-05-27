'use client';

import { RefreshCw, Timer } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useQuoteCountdown } from '@/hooks/useQuoteCountdown';
import { cn } from '@/lib/utils';
import { Progress } from '@/components/ui/progress';

interface QuoteCountdownTimerProps {
  expiresAtMs: number | undefined;
  ttlSeconds: number | undefined;
  onRefresh: () => void;
  isLoading?: boolean;
  className?: string;
}

/**
 * Visual countdown timer for quote validity.
 * Displays a progress bar and remaining seconds.
 * Triggers a refresh CTA when the quote expires.
 */
export function QuoteCountdownTimer({
  expiresAtMs,
  ttlSeconds,
  onRefresh,
  isLoading,
  className,
}: QuoteCountdownTimerProps) {
  const totalTtlMs = (ttlSeconds ?? 5.5) * 1000;
  const { remainingSeconds, isExpired, progress } = useQuoteCountdown(
    expiresAtMs,
    totalTtlMs
  );

  if (!expiresAtMs) return null;

  return (
    <div className={cn('flex flex-col gap-1.5 w-full', className)}>
      <div className="flex items-center justify-between text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
        <div className="flex items-center gap-1">
          <Timer className={cn('w-3 h-3', isExpired ? 'text-destructive' : 'text-primary')} />
          <span>
            {isExpired ? 'Quote Expired' : `Valid for ${remainingSeconds}s`}
          </span>
        </div>
        {!isExpired && (
          <span className="opacity-70">
            {Math.round(progress * 100)}%
          </span>
        )}
      </div>
      
      <div className="relative h-1 w-full bg-muted rounded-full overflow-hidden">
        <Progress 
          value={progress * 100} 
          className={cn(
            'h-full transition-all duration-300',
            isExpired ? 'bg-destructive' : progress < 0.3 ? 'bg-amber-500' : 'bg-primary'
          )}
        />
      </div>

      {isExpired && (
        <div className="mt-1 animate-in fade-in slide-in-from-top-1 duration-300">
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            disabled={isLoading}
            className="w-full h-8 text-xs gap-2 border-primary/20 hover:border-primary/50 hover:bg-primary/5 text-primary"
          >
            <RefreshCw className={cn('w-3 h-3', isLoading && 'animate-spin')} />
            Refresh Quote
          </Button>
        </div>
      )}
    </div>
  );
}
