'use client';

import { Check } from 'lucide-react';
import { cn } from '@/lib/utils';

const STEPS = [
  { id: 'burn', label: 'Burn', detail: 'Lock USDC on source chain' },
  { id: 'attest', label: 'Attest', detail: 'Circle attestation relay' },
  { id: 'mint', label: 'Mint', detail: 'Release on destination' },
] as const;

export type CctpStepId = (typeof STEPS)[number]['id'];

interface CctpStepRailProps {
  previewOnly?: boolean;
  activeStep?: CctpStepId | null;
  completedSteps?: CctpStepId[];
  className?: string;
}

export function CctpStepRail({
  previewOnly = true,
  activeStep = null,
  completedSteps = [],
  className,
}: CctpStepRailProps) {
  const completed = new Set(completedSteps);

  return (
    <ol
      className={cn('flex flex-col gap-2 sm:flex-row sm:items-stretch', className)}
      aria-label="CCTP protocol steps"
    >
      {STEPS.map((step, index) => {
        const isComplete = !previewOnly && completed.has(step.id);
        const isActive = !previewOnly && !isComplete && activeStep === step.id;
        return (
          <li
            key={step.id}
            className={cn(
              'relative flex min-h-11 flex-1 flex-col rounded-xl border p-3',
              previewOnly
                ? 'border-dashed border-border/40 bg-background/50'
                : isComplete
                  ? 'border-primary/40 bg-primary/10'
                  : isActive
                    ? 'border-signal/50 bg-signal/10'
                    : 'border-border/40 bg-background/50',
            )}
            aria-current={isActive ? 'step' : undefined}
            data-state={
              previewOnly
                ? 'preview'
                : isComplete
                  ? 'complete'
                  : isActive
                    ? 'active'
                    : 'pending'
            }
          >
            <span className="font-mono text-[10px] uppercase tracking-wider text-primary">
              Step {index + 1}
            </span>
            <span className="inline-flex items-center gap-1.5 text-sm font-semibold">
              {isComplete && <Check className="h-3.5 w-3.5 text-primary" aria-hidden />}
              {step.label}
            </span>
            <span className="text-xs text-muted-foreground">{step.detail}</span>
            {previewOnly && (
              <span className="mt-1 text-[10px] uppercase tracking-wide text-muted-foreground">
                Preview — not live
              </span>
            )}
            {!previewOnly && isActive && (
              <span className="mt-1 text-[10px] uppercase tracking-wide text-signal">
                In progress
              </span>
            )}
            {!previewOnly && isComplete && (
              <span className="mt-1 text-[10px] uppercase tracking-wide text-primary">
                Complete
              </span>
            )}
          </li>
        );
      })}
    </ol>
  );
}

export function cctpActiveStepFromSaga(
  status?: string,
): CctpStepId | null {
  if (!status) return 'burn';
  if (status === 'completed') return null;
  if (status === 'burn_submitted') return 'attest';
  if (
    status === 'awaiting_attestation' ||
    status === 'attestation_failed'
  ) {
    return 'attest';
  }
  if (
    status === 'attestation_ready' ||
    status === 'mint_prepared' ||
    status === 'mint_submitted' ||
    status === 'mint_failed_retryable'
  ) {
    return 'mint';
  }
  return 'burn';
}

export function cctpCompletedStepsFromSaga(status?: string): CctpStepId[] {
  if (!status) return [];
  if (status === 'completed') return ['burn', 'attest', 'mint'];
  if (
    status === 'mint_prepared' ||
    status === 'mint_submitted' ||
    status === 'mint_failed_retryable' ||
    status === 'attestation_ready'
  ) {
    return ['burn', 'attest'];
  }
  if (
    status === 'awaiting_attestation' ||
    status === 'attestation_failed' ||
    status === 'burn_submitted'
  ) {
    return ['burn'];
  }
  return [];
}
