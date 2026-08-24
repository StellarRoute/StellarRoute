'use client';

import { cn } from '@/lib/utils';
import type { OfframpMode } from '@/lib/offramp/types';

interface OfframpModeToggleProps {
  mode: OfframpMode;
  onChange: (mode: OfframpMode) => void;
  className?: string;
}

const MODES: Array<{
  id: OfframpMode;
  title: string;
  description: string;
}> = [
  {
    id: 'direct',
    title: 'Stellar USDC',
    description: 'Cash out USDC you already hold on Stellar.',
  },
  {
    id: 'bridge',
    title: 'Bridge + offramp',
    description: 'Pick any supported coin, bridge to Stellar, then Naira.',
  },
];

export function OfframpModeToggle({
  mode,
  onChange,
  className,
}: OfframpModeToggleProps) {
  return (
    <div
      className={cn('grid gap-3 sm:grid-cols-2', className)}
      role="radiogroup"
      aria-label="Offramp path"
      data-testid="offramp-mode-toggle"
    >
      {MODES.map((item) => {
        const selected = mode === item.id;
        return (
          <button
            key={item.id}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(item.id)}
            className={cn(
              'group relative overflow-hidden rounded-2xl border px-5 py-4 text-left transition-all duration-300',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
              selected
                ? 'border-primary/50 bg-primary/8 shadow-[inset_0_1px_0_color-mix(in_srgb,var(--primary)_35%,transparent)]'
                : 'border-border/70 bg-card/40 hover:border-primary/30 hover:bg-card/70',
            )}
            data-testid={`offramp-mode-${item.id}`}
          >
            <span
              className={cn(
                'mb-2 inline-flex font-mono text-[10px] font-semibold uppercase tracking-[0.22em]',
                selected ? 'text-primary' : 'text-muted-foreground',
              )}
            >
              {item.id === 'direct' ? 'Fastest' : 'Any coin'}
            </span>
            <span className="block font-display text-lg font-semibold tracking-tight text-foreground">
              {item.title}
            </span>
            <span className="mt-1 block text-sm leading-relaxed text-muted-foreground">
              {item.description}
            </span>
            {selected && (
              <span
                aria-hidden
                className="pointer-events-none absolute -right-6 -top-6 size-20 rounded-full bg-primary/15 blur-2xl"
              />
            )}
          </button>
        );
      })}
    </div>
  );
}
