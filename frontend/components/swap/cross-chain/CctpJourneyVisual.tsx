'use client';

import { Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useReducedMotion } from '@/hooks/useReducedMotion';
import type { CctpStepId } from './CctpStepRail';
import { cctpActiveStepFromSaga, cctpCompletedStepsFromSaga } from './CctpStepRail';

const HOPS: Array<{ id: CctpStepId; label: string; hint: string }> = [
  { id: 'burn', label: 'Lock', hint: 'Source' },
  { id: 'attest', label: 'Confirm', hint: 'Waiting' },
  { id: 'mint', label: 'Receive', hint: 'Destination' },
];

export interface CctpJourneyVisualProps {
  status?: string | null;
  className?: string;
}

export function CctpJourneyVisual({
  status,
  className,
}: CctpJourneyVisualProps) {
  const prefersReducedMotion = useReducedMotion();
  const active = cctpActiveStepFromSaga(status ?? undefined);
  const completed = new Set(cctpCompletedStepsFromSaga(status ?? undefined));
  const isComplete = status === 'completed';
  const packetIndex = isComplete
    ? HOPS.length - 1
    : Math.max(
        0,
        HOPS.findIndex((h) => h.id === active),
      );
  const progressPct = ((packetIndex + (isComplete ? 1 : 0.45)) / HOPS.length) * 100;

  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-2xl border border-border/40',
        'bg-[radial-gradient(ellipse_at_20%_0%,color-mix(in_srgb,var(--primary)_18%,transparent),transparent_55%),radial-gradient(ellipse_at_90%_100%,color-mix(in_srgb,var(--signal)_12%,transparent),transparent_50%),color-mix(in_srgb,var(--card)_70%,transparent)]',
        'px-4 py-5',
        className,
      )}
      data-testid="cctp-journey-visual"
      data-status={status ?? 'idle'}
      aria-label="Transfer progress"
    >
      <div
        className="cctp-journey-grid pointer-events-none absolute inset-0 opacity-40"
        aria-hidden
      />

      <div className="relative z-[1] grid grid-cols-3 gap-2">
        {HOPS.map((hop, index) => {
          const done = completed.has(hop.id) || isComplete;
          const current = !isComplete && active === hop.id;
          return (
            <div
              key={hop.id}
              className="flex flex-col items-center gap-1.5 min-w-0"
            >
              <div
                className={cn(
                  'relative flex h-11 w-11 items-center justify-center rounded-full border text-xs font-bold',
                  done && 'border-primary/50 bg-primary text-primary-foreground',
                  current &&
                    'border-signal/60 bg-signal/15 text-signal shadow-[0_0_0_4px] shadow-signal/15',
                  !done &&
                    !current &&
                    'border-border/50 bg-background/70 text-muted-foreground',
                )}
                aria-current={current ? 'step' : undefined}
              >
                {done ? (
                  <Check className="h-4 w-4" aria-hidden />
                ) : (
                  <span>{index + 1}</span>
                )}
                {current && !prefersReducedMotion && (
                  <span className="cctp-journey-ring absolute inset-[-4px] rounded-full" />
                )}
              </div>
              <div className="text-center min-w-0">
                <p
                  className={cn(
                    'text-xs font-semibold truncate',
                    current || done ? 'text-foreground' : 'text-muted-foreground',
                  )}
                >
                  {hop.label}
                </p>
                <p className="text-[10px] text-muted-foreground truncate">
                  {hop.hint}
                </p>
              </div>
            </div>
          );
        })}
      </div>

      <div className="relative z-[1] mt-5 h-1.5 overflow-hidden rounded-full bg-muted/40">
        <div
          className={cn(
            'absolute inset-y-0 left-0 rounded-full bg-gradient-to-r from-primary/50 via-primary to-signal',
            !prefersReducedMotion && !isComplete && 'cctp-journey-progress',
          )}
          style={{ width: `${Math.min(100, progressPct)}%` }}
        />
        {!prefersReducedMotion && !isComplete && (
          <span
            className="cctp-journey-packet"
            style={{ left: `calc(${Math.min(96, progressPct)}% - 5px)` }}
          />
        )}
      </div>

      <p className="relative z-[1] mt-4 text-center text-xs text-muted-foreground">
        {isComplete
          ? 'Transfer complete — USDC arrived on destination.'
          : active === 'burn'
            ? 'Locking on source — confirm when your wallet prompts.'
            : active === 'attest'
              ? 'In transit — waiting for confirmation.'
              : active === 'mint'
                ? 'Almost there — confirm receive on destination.'
                : 'Getting ready…'}
      </p>
    </div>
  );
}
