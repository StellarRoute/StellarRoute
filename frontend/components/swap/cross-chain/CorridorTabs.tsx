'use client';

import { useMemo, useRef, useCallback } from 'react';
import { cn } from '@/lib/utils';
import type { CorridorId, CorridorSelectionId } from '@/lib/cross-chain/types';
import { UNMATCHED_CORRIDOR_ID } from '@/lib/cross-chain/corridors';
import { useCorridorCatalog } from '@/hooks/useCorridorCatalog';
import { useFeatureFlag } from '@/hooks/useFeatureFlag';

interface CorridorTabsProps {
  activeId: CorridorSelectionId;
  onSelect: (id: CorridorId) => void;
  disabled?: boolean;
}

export function CorridorTabs({ activeId, onSelect, disabled }: CorridorTabsProps) {
  const { corridors } = useCorridorCatalog();
  const { enabled: keyboardNavEnabled } = useFeatureFlag('corridor_keyboard_nav');
  const isUnmatched = activeId === UNMATCHED_CORRIDOR_ID;
  const listRef = useRef<HTMLDivElement>(null);

  const { ready, soon } = useMemo(() => {
    const readyCorridors = corridors.filter((c) => c.executable);
    const soonCorridors = corridors.filter((c) => !c.executable);
    return { ready: readyCorridors, soon: soonCorridors };
  }, [corridors]);

  const allIds: CorridorId[] = useMemo(
    () => [...ready, ...soon].map((c) => c.id),
    [ready, soon]
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (!keyboardNavEnabled || disabled) return;
      const tabs = listRef.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]:not([disabled])');
      if (!tabs || tabs.length === 0) return;

      const focused = document.activeElement as HTMLButtonElement | null;
      const tabArray = Array.from(tabs);
      const currentIndex = focused ? tabArray.indexOf(focused) : -1;

      let nextIndex: number | undefined;
      if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
        event.preventDefault();
        nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % tabArray.length;
      } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
        event.preventDefault();
        nextIndex = currentIndex <= 0 ? tabArray.length - 1 : currentIndex - 1;
      } else if (event.key === 'Home') {
        event.preventDefault();
        nextIndex = 0;
      } else if (event.key === 'End') {
        event.preventDefault();
        nextIndex = tabArray.length - 1;
      }

      if (nextIndex !== undefined) {
        tabArray[nextIndex].focus();
        const id = tabArray[nextIndex].dataset.corridorId as CorridorId | undefined;
        if (id) onSelect(id);
      }
    },
    [keyboardNavEnabled, disabled, onSelect]
  );

  return (
    <nav aria-label="Cross-chain corridors" className="space-y-2.5">
      <div className="flex items-baseline justify-between gap-3">
        <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
          Quick routes
        </p>
        {isUnmatched && (
          <p className="text-xs text-muted-foreground" id="corridor-tab-unmatched">
            Custom pair
          </p>
        )}
      </div>

      <div
        ref={listRef}
        role="tablist"
        className="flex flex-wrap items-center gap-2"
        aria-orientation="horizontal"
        onKeyDown={keyboardNavEnabled ? handleKeyDown : undefined}
      >
        {ready.map((corridor) => {
          const selected = !isUnmatched && corridor.id === activeId;
          return (
            <CorridorChip
              key={corridor.id}
              id={corridor.id}
              label={corridor.label}
              selected={selected}
              disabled={disabled}
              onSelect={onSelect}
              tone="ready"
            />
          );
        })}

        {soon.length > 0 && (
          <span
            className="mx-0.5 hidden h-4 w-px bg-border/60 sm:inline-block"
            aria-hidden
          />
        )}

        {soon.map((corridor) => {
          const selected = !isUnmatched && corridor.id === activeId;
          return (
            <CorridorChip
              key={corridor.id}
              id={corridor.id}
              label={corridor.label}
              selected={selected}
              disabled={disabled}
              onSelect={onSelect}
              tone="soon"
            />
          );
        })}
      </div>
    </nav>
  );
}

function CorridorChip({
  id,
  label,
  selected,
  disabled,
  onSelect,
  tone,
}: {
  id: CorridorId;
  label: string;
  selected: boolean;
  disabled?: boolean;
  onSelect: (id: CorridorId) => void;
  tone: 'ready' | 'soon';
}) {
  return (
    <button
      type="button"
      role="tab"
      id={`corridor-tab-${id}`}
      aria-selected={selected}
      aria-controls={`corridor-panel-${id}`}
      onClick={() => onSelect(id)}
      disabled={disabled}
      data-corridor-id={id}
      className={cn(
        'min-h-11 rounded-full border px-3.5 py-2 text-sm font-medium transition-colors',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
        selected
          ? 'border-primary/60 bg-primary/15 text-foreground'
          : tone === 'ready'
            ? 'border-border/45 bg-background/50 text-foreground/85 hover:border-border hover:bg-muted/25'
            : 'border-transparent bg-transparent text-muted-foreground/80 hover:border-border/40 hover:bg-muted/15 hover:text-muted-foreground',
      )}
      data-testid={`corridor-tab-${id}`}
    >
      {label}
      {tone === 'soon' && (
        <span className="ml-1.5 font-mono text-[9px] uppercase tracking-wider opacity-60">
          Soon
        </span>
      )}
    </button>
  );
}
