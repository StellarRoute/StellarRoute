'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface AmountPresetsProps {
  balance: string | null;
  decimals?: number;
  onSelect: (percentage: number) => void;
  disabled?: boolean;
  className?: string;
}

const PRESETS = [
  { label: '25%', value: 0.25 },
  { label: '50%', value: 0.5 },
  { label: '75%', value: 0.75 },
  { label: '100%', value: 1.0 },
];

/**
 * Quick-fill percentage buttons for swap amounts
 * Respects asset decimals and rounding rules
 */
export function AmountPresets({
  balance,
  decimals = 7,
  onSelect,
  disabled = false,
  className,
}: AmountPresetsProps) {
  const isDisabled = disabled || !balance || parseFloat(balance) === 0;

  const handlePresetClick = (percentage: number) => {
    if (isDisabled) return;
    onSelect(percentage);
  };

  return (
    <div
      className={cn('flex items-center gap-1.5', className)}
      role="group"
      aria-label="Amount presets"
    >
      {PRESETS.map(({ label, value }) => (
        <Button
          key={label}
          type="button"
          variant="outline"
          size="sm"
          onClick={() => handlePresetClick(value)}
          disabled={isDisabled}
          className={cn(
            'h-7 px-2.5 text-[11px] font-semibold rounded-lg',
            'border-border/40 hover:border-primary/40 hover:bg-primary/5',
            'transition-all duration-200',
            'disabled:opacity-40 disabled:cursor-not-allowed'
          )}
          aria-label={`Set amount to ${label} of balance`}
        >
          {label}
        </Button>
      ))}
    </div>
  );
}
