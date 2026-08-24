'use client';

import { useEffect, useRef, useState } from 'react';
import { AlertCircle, AlertTriangle } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { useSwapI18n } from '@/lib/swap-i18n';

interface SlippageSettingsProps {
  value: number;
  onChange: (value: number) => void;
}

const PRESETS = [
  { label: 'Safe', value: 0.1 },
  { label: 'Balanced', value: 0.5 },
  { label: 'Aggressive', value: 1 },
];

export function SlippageSettings({ value, onChange }: SlippageSettingsProps) {
  const { t } = useSwapI18n();
  const [customValue, setCustomValue] = useState(
    PRESETS.some((preset) => preset.value === value) ? '' : String(value)
  );
  const lastEmittedValue = useRef<number | null>(null);
  const isLow = value < 0.1;
  const isHigh = value > 5;

  useEffect(() => {
    if (lastEmittedValue.current === value) {
      lastEmittedValue.current = null;
      return;
    }
    setCustomValue(
      PRESETS.some((preset) => preset.value === value) ? '' : String(value)
    );
  }, [value]);

  const updateCustomValue = (rawValue: string) => {
    setCustomValue(rawValue);
    const parsed = Number.parseFloat(rawValue);
    if (Number.isFinite(parsed)) {
      const clamped = Math.max(0.01, Math.min(50, parsed));
      lastEmittedValue.current = clamped;
      onChange(clamped);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="text-sm font-semibold tracking-tight">
          {t('swap.settings.slippageTolerance')}
        </span>
        <span
          className={cn(
            'rounded-full px-2 py-0.5 text-xs font-bold',
            isHigh
              ? 'bg-destructive/10 text-destructive'
              : 'bg-primary/10 text-primary'
          )}
        >
          {value}%
        </span>
      </div>

      <div className="flex flex-wrap gap-2">
        {PRESETS.map((preset) => (
          <Button
            key={preset.label}
            type="button"
            variant={value === preset.value ? 'default' : 'outline'}
            size="sm"
            onClick={() => {
              setCustomValue('');
              lastEmittedValue.current = preset.value;
              onChange(preset.value);
            }}
            className="h-10 flex-1 font-bold"
          >
            {preset.label}
          </Button>
        ))}

        <div className="relative min-w-[120px] flex-[1.5]">
          <Input
            type="number"
            step="0.01"
            min="0.01"
            max="50"
            aria-label={`Custom ${t('swap.settings.slippageTolerance')} percentage`}
            className={cn(
              'h-10 pr-8 text-right font-bold',
              customValue && 'border-primary ring-1 ring-primary/20'
            )}
            placeholder={t('settings.slippage.custom')}
            value={customValue}
            onChange={(event) => updateCustomValue(event.target.value)}
          />
          <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs font-bold text-muted-foreground">
            %
          </span>
        </div>
      </div>

      {isLow && (
        <div className="flex items-center gap-2 rounded-xl border border-yellow-500/20 bg-yellow-500/10 p-3 text-[11px] font-medium text-yellow-600 dark:text-yellow-400">
          <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
          <p>{t('settings.slippage.lowWarning', { value })}</p>
        </div>
      )}

      {isHigh && (
        <div className="flex items-center gap-2 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-[11px] font-medium text-destructive">
          <AlertCircle className="h-3.5 w-3.5 shrink-0" />
          <p>{t('settings.slippage.highWarning')}</p>
        </div>
      )}
    </div>
  );
}
