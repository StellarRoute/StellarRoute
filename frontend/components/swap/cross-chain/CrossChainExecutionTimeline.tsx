'use client';

import type { ExecutionTimelineStep } from '@/lib/cross-chain/types';
import { cn } from '@/lib/utils';
import { ExternalLink, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface CrossChainExecutionTimelineProps {
  steps: ExecutionTimelineStep[];
  className?: string;
}

const STATUS_LABEL: Record<ExecutionTimelineStep['status'], string> = {
  unavailable: 'Unavailable',
  pending: 'Pending',
  active: 'In progress',
  complete: 'Complete',
  failed: 'Failed',
};

export function CrossChainExecutionTimeline({
  steps,
  className,
}: CrossChainExecutionTimelineProps) {
  const activeIndex = steps.findIndex((s) => s.status === 'active');

  return (
    <section
      aria-label="Execution timeline"
      className={cn('space-y-3', className)}
      data-testid="execution-timeline"
    >
      <h3 className="font-semibold text-foreground">Execution timeline</h3>
      <ol className="space-y-2" role="list">
        {steps.map((step, index) => {
          const isCurrent = index === activeIndex || step.status === 'active';
          return (
            <li
              key={step.id}
              role="listitem"
              aria-current={isCurrent ? 'step' : undefined}
              className={cn(
                'rounded-xl border p-3',
                step.status === 'active'
                  ? 'border-primary/40 bg-primary/8'
                  : 'border-border/40 bg-card/40',
                step.status === 'unavailable' && 'opacity-70'
              )}
              data-testid={`timeline-step-${step.id}`}
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <p className="text-sm font-semibold">{step.label}</p>
                  <p className="text-xs text-muted-foreground">{step.description}</p>
                  <p className="mt-1 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                    {STATUS_LABEL[step.status]}
                  </p>
                </div>
                <div className="flex flex-col gap-1">
                  {step.href && (
                    <a
                      href={step.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex min-h-11 items-center gap-1 text-xs text-primary hover:underline"
                    >
                      View tx
                      <ExternalLink className="h-3 w-3" aria-hidden />
                    </a>
                  )}
                  {step.retryable && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="min-h-11 text-xs"
                      disabled
                      title="Retry requires live backend execution state"
                    >
                      <RotateCcw className="h-3 w-3 mr-1" aria-hidden />
                      Retry (preview)
                    </Button>
                  )}
                </div>
              </div>
              {step.supportReference && (
                <p className="mt-2 font-mono text-[10px] text-muted-foreground">
                  Support ref: {step.supportReference}
                </p>
              )}
            </li>
          );
        })}
      </ol>
      <p className="text-xs text-muted-foreground">
        Timeline reflects durable backend states when execution is live. Preview
        corridors do not submit transactions.
      </p>
    </section>
  );
}
