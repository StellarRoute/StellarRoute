'use client';

import { cn } from '@/lib/utils';
import type { OfframpRouteStep } from '@/lib/offramp/types';

interface OfframpRouteRailProps {
  steps: OfframpRouteStep[];
  className?: string;
}

export function OfframpRouteRail({ steps, className }: OfframpRouteRailProps) {
  return (
    <ol
      className={cn('space-y-0', className)}
      data-testid="offramp-route-rail"
      aria-label="Offramp route"
    >
      {steps.map((step, index) => {
        const isLast = index === steps.length - 1;
        return (
          <li key={`${step.id}-${index}`} className="relative flex gap-4 pb-6 last:pb-0">
            {!isLast && (
              <span
                aria-hidden
                className="absolute left-[11px] top-7 h-[calc(100%-1.25rem)] w-px bg-border"
              />
            )}
            <span
              aria-hidden
              className={cn(
                'relative z-[1] mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full border text-[10px] font-bold',
                step.active
                  ? 'border-primary bg-primary text-primary-foreground'
                  : 'border-border bg-muted text-muted-foreground',
              )}
            >
              {index + 1}
            </span>
            <div className="min-w-0 pt-0.5">
              <p className="font-medium text-foreground">{step.label}</p>
              <p className="mt-0.5 text-sm leading-relaxed text-muted-foreground">
                {step.detail}
              </p>
            </div>
          </li>
        );
      })}
    </ol>
  );
}
