'use client';

import * as React from 'react';
import { useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';
import { buildOfframpRedirectUrl } from '@/lib/ai/client';
import { capMessage, exceedsCap } from '@/lib/ai/cap';
import type { Intent } from '@/lib/ai/parse';
import { loadTranscript, saveTranscript } from '@/lib/ai/transcript';
import { CheckCircle2 } from 'lucide-react';

export interface PlanRunnerProps {
  steps: Intent[];
  /** Local per-confirm USDC cap (AI-31); null means no extra block. */
  usdcCap?: number | null;
  /** Called when a step is handed off, before any navigation. */
  onHandoff?: (step: Intent, index: number) => void;
}

export function stepLabel(step: Intent): string {
  switch (step.type) {
    case 'swap':
      return `Swap ${step.amount} ${step.asset} to ${step.destination ?? ''}`.trim();
    case 'send':
      return `Send ${step.amount} ${step.asset} to ${step.recipient ?? ''}`.trim();
    case 'bridge':
      return step.destination
        ? `Bridge ${step.amount} ${step.asset} to ${step.destination}`
        : `Bridge ${step.amount} ${step.asset} from ${step.source ?? ''}`.trim();
    case 'cash_out':
      return `Cash out ${step.amount} ${step.asset}${step.currency ? ` to ${step.currency}` : ''}`;
    case 'pay':
      return `Pay ${step.amount} ${step.asset} ${step.recurrence ?? ''} to ${step.recipient ?? ''}`.trim();
    case 'receive':
      return 'Receive payment';
  }
}

/** Existing page that takes over a step. The runner itself never calls a wallet. */
function handoffUrl(step: Intent): string | null {
  switch (step.type) {
    case 'swap':
      return '/swap';
    case 'bridge':
      return '/cross-chain-swap';
    case 'cash_out':
      return buildOfframpRedirectUrl({ kind: 'offramp', amount: step.amount, fromAsset: step.asset });
    default:
      return null;
  }
}

export function PlanRunner({ steps, usdcCap = null, onHandoff }: PlanRunnerProps) {
  const router = useRouter();
  const [doneCount, setDoneCount] = React.useState(0);

  const handleConfirm = (index: number) => {
    if (index !== doneCount) return;
    const step = steps[index];
    const now = Date.now();

    saveTranscript([
      ...loadTranscript(),
      {
        id: `plan-step-${index + 1}-${now}`,
        intentId: `plan-step-${index + 1}`,
        content: `Step ${index + 1} done: ${stepLabel(step)}`,
        timestamp: now,
      },
    ]);
    setDoneCount(index + 1);
    onHandoff?.(step, index);

    const url = handoffUrl(step);
    if (url) router.push(url);
  };

  return (
    <ol data-testid="plan-runner" className="space-y-3">
      {steps.map((step, index) => {
        const done = index < doneCount;
        const current = index === doneCount;
        const overCap =
          current && step.type !== 'receive' && exceedsCap(step.amount, step.asset, usdcCap);

        return (
          <li
            key={index}
            data-testid={`plan-step-${index + 1}`}
            data-state={done ? 'done' : current ? 'current' : 'pending'}
            className={`rounded-lg border p-4 space-y-3 ${
              current ? 'border-border bg-card shadow-sm' : 'border-dashed border-border opacity-60'
            }`}
          >
            <div className="flex items-center gap-2 text-sm font-medium">
              {done ? (
                <CheckCircle2 className="h-4 w-4 text-emerald-500" aria-label="Done" />
              ) : (
                <span className="h-4 w-4 rounded-full border border-muted-foreground" aria-hidden />
              )}
              <span className="text-xs uppercase tracking-wider text-muted-foreground">
                Step {index + 1}
              </span>
              <span data-testid={`plan-step-${index + 1}-label`}>{stepLabel(step)}</span>
            </div>

            {current && (
              <>
                {overCap && usdcCap !== null && (
                  <p className="text-xs text-destructive" data-testid="cap-exceeded">
                    {capMessage(usdcCap)}
                  </p>
                )}
                <Button
                  size="sm"
                  onClick={() => handleConfirm(index)}
                  disabled={overCap}
                  data-testid={`plan-step-${index + 1}-confirm`}
                  className="gap-1.5"
                >
                  <CheckCircle2 className="h-4 w-4" />
                  Confirm
                </Button>
              </>
            )}
          </li>
        );
      })}
    </ol>
  );
}
