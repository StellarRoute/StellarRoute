'use client';

import { useEffect, useState } from 'react';
import { Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useReducedMotion } from '@/hooks/useReducedMotion';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';
import { useSwapI18n } from '@/lib/swap-i18n';
import { DialogDescription, DialogTitle } from '@/components/ui/dialog';

export type SwapWaitPhase = 'pending' | 'submitted';

export interface SwapWaitingStateProps {
  phase: SwapWaitPhase;
  tradeParams?: TradeParams;
  className?: string;
}

function formatElapsed(totalSeconds: number): string {
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export function SwapWaitingState({
  phase,
  tradeParams,
  className,
}: SwapWaitingStateProps) {
  const { t } = useSwapI18n();
  const prefersReducedMotion = useReducedMotion();
  const [elapsedSec, setElapsedSec] = useState(0);
  const isPending = phase === 'pending';

  useEffect(() => {
    setElapsedSec(0);
    const id = window.setInterval(() => {
      setElapsedSec((prev) => prev + 1);
    }, 1000);
    return () => window.clearInterval(id);
  }, [phase]);

  const heading = isPending
    ? t('swap.confirm.pending.heading')
    : t('swap.confirm.submitted.heading');
  const description = isPending
    ? t('swap.confirm.pending.description')
    : t('swap.confirm.submitted.description');
  const tip = isPending
    ? t('swap.confirm.pending.tip')
    : t('swap.confirm.submitted.tip');

  return (
    <div
      className={cn('flex flex-col items-center gap-6 text-center', className)}
      data-testid="swap-waiting-state"
      data-phase={phase}
    >
      <WaitingBeacon
        phase={phase}
        prefersReducedMotion={prefersReducedMotion}
      />

      <div className="space-y-2 max-w-[20rem]">
        <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary">
          {isPending
            ? t('swap.confirm.wait.phase.wallet')
            : t('swap.confirm.wait.phase.network')}
        </p>
        <DialogTitle className="font-display text-2xl font-bold tracking-tight text-balance">
          {heading}
        </DialogTitle>
        <DialogDescription className="text-sm text-muted-foreground leading-relaxed text-pretty">
          {description}
        </DialogDescription>
      </div>

      <ol
        className="w-full grid grid-cols-[1fr_auto_1fr] items-center gap-2"
        aria-label={t('swap.confirm.wait.stepsLabel')}
      >
        <WaitStep
          index={1}
          label={t('swap.confirm.wait.step.sign')}
          state={isPending ? 'active' : 'done'}
        />
        <div
          className={cn(
            'h-px w-6 sm:w-10 self-start mt-4',
            isPending ? 'bg-border' : 'bg-primary/50'
          )}
          aria-hidden
        />
        <WaitStep
          index={2}
          label={t('swap.confirm.wait.step.confirm')}
          state={isPending ? 'upcoming' : 'active'}
        />
      </ol>

      <p
        className="w-full rounded-2xl border border-border/40 bg-muted/25 px-4 py-3 text-left text-sm text-muted-foreground leading-relaxed"
        data-testid="swap-waiting-tip"
      >
        <span className="block text-[11px] font-semibold uppercase tracking-[0.18em] text-foreground/70 mb-1">
          {t('swap.confirm.wait.tipLabel')}
        </span>
        {tip}
      </p>

      {tradeParams && (
        <div className="w-full rounded-2xl border border-border/20 bg-muted/30 p-4 space-y-2 text-sm text-left">
          <div className="flex justify-between gap-3 min-w-0">
            <span className="text-muted-foreground shrink-0">
              {t('swap.confirm.summary.youPay')}
            </span>
            <span className="font-medium text-right break-words min-w-0">
              {tradeParams.fromAmount} {tradeParams.fromAsset}
            </span>
          </div>
          <div className="flex justify-between gap-3 min-w-0">
            <span className="text-muted-foreground shrink-0">
              {t('swap.confirm.summary.youReceive')}
            </span>
            <span className="font-medium text-right break-words min-w-0">
              {tradeParams.toAmount} {tradeParams.toAsset}
            </span>
          </div>
        </div>
      )}

      <p
        className="text-xs tabular-nums text-muted-foreground/80"
        data-testid="swap-waiting-elapsed"
        aria-live="off"
      >
        {t('swap.confirm.wait.elapsed', { time: formatElapsed(elapsedSec) })}
      </p>
    </div>
  );
}

function WaitStep({
  index,
  label,
  state,
}: {
  index: number;
  label: string;
  state: 'done' | 'active' | 'upcoming';
}) {
  return (
    <li
      className={cn(
        'flex flex-col items-center gap-2 min-w-0',
        state === 'upcoming' && 'opacity-45'
      )}
      aria-current={state === 'active' ? 'step' : undefined}
    >
      <span
        className={cn(
          'flex h-8 w-8 items-center justify-center rounded-full border text-xs font-bold',
          state === 'done' &&
            'border-primary/40 bg-primary text-primary-foreground',
          state === 'active' &&
            'border-signal/50 bg-signal/15 text-signal shadow-[0_0_0_4px] shadow-signal/10',
          state === 'upcoming' && 'border-border bg-muted/40 text-muted-foreground'
        )}
      >
        {state === 'done' ? <Check className="h-3.5 w-3.5" aria-hidden /> : index}
      </span>
      <span
        className={cn(
          'text-[11px] font-medium leading-tight text-balance',
          state === 'active' ? 'text-foreground' : 'text-muted-foreground'
        )}
      >
        {label}
      </span>
    </li>
  );
}

function WaitingBeacon({
  phase,
  prefersReducedMotion,
}: {
  phase: SwapWaitPhase;
  prefersReducedMotion: boolean;
}) {
  const isPending = phase === 'pending';

  return (
    <div
      className={cn(
        'relative flex h-28 w-28 items-center justify-center',
        'rounded-full',
        isPending ? 'tcm-wait-wallet' : 'tcm-wait-network'
      )}
      aria-hidden
    >
      <span className="tcm-wait-ring tcm-wait-ring-outer" />
      <span className="tcm-wait-ring tcm-wait-ring-mid" />
      <span
        data-testid="tcm-spinner"
        className={cn(
          'tcm-wait-sweep',
          !prefersReducedMotion && 'tcm-orbit-spin'
        )}
      />
      <span
        className={cn(
          'relative z-10 flex h-14 w-14 items-center justify-center rounded-full',
          'border border-border/50 bg-background/90 backdrop-blur-sm',
          'shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]',
          isPending ? 'text-signal' : 'text-primary'
        )}
      >
        <span
          className={cn(
            'h-2.5 w-2.5 rounded-full',
            isPending ? 'bg-signal' : 'bg-primary',
            !prefersReducedMotion && 'tcm-signal-pulse'
          )}
        />
      </span>
    </div>
  );
}
