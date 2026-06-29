'use client';

import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  maxDecimalsForSellAsset,
  parseSellAmount,
  clampToMaxDecimals,
  normalizeDecimalString,
} from '@/lib/amount-input';
import { useState, useCallback, useEffect, useId } from 'react';
import { AmountPresets } from './AmountPresets';

interface AmountInputProps {
  value: string;
  onChange?: (value: string) => void;
  onMax?: () => void;
  onPresetSelect?: (percentage: number) => void;
  placeholder?: string;
  disabled?: boolean;
  readOnly?: boolean;
  className?: string;
  label?: string;
  balance?: string;
  balanceLoading?: boolean;
  balanceError?: boolean;
  showMax?: boolean;
  showPresets?: boolean;
  /**
   * Explicit decimal precision for this asset.
   * When provided, overrides the heuristic from `assetId`.
   * Accepts 0–255 (Stellar/Soroban range).
   */
  decimals?: number;
  /**
   * Canonical asset identifier ("native" or "CODE:ISSUER").
   * Used to derive adaptive precision when `decimals` is not supplied.
   */
  assetId?: string;
}

/**
 * Resolve effective max decimals from props.
 * Priority: explicit `decimals` > `assetId` heuristic > default (7).
 */
function resolveMaxDecimals(decimals?: number, assetId?: string): number {
  return maxDecimalsForSellAsset(assetId ?? 'native', decimals);
}

export function AmountInput({
  value,
  onChange,
  onMax,
  onPresetSelect,
  placeholder = '0.00',
  disabled = false,
  readOnly = false,
  className,
  label,
  balance,
  balanceLoading = false,
  balanceError = false,
  showMax = true,
  showPresets = false,
  decimals,
  assetId,
}: AmountInputProps) {
  const maxDecimals = resolveMaxDecimals(decimals, assetId);
  const [internalValue, setInternalValue] = useState(value);
  const [precisionError, setPrecisionError] = useState<string | null>(null);
  const inputId = useId();

  useEffect(() => {
    setInternalValue(value);
    // Re-validate when value changes externally
    if (value !== '') {
      const result = parseSellAmount(value, maxDecimals);
      setPrecisionError(
        result.status === 'precision_exceeded' ? result.message : null
      );
    } else {
      setPrecisionError(null);
    }
  }, [value, maxDecimals]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const raw = e.target.value.replace(/,/g, '.');

      if (raw === '') {
        setInternalValue('');
        setPrecisionError(null);
        onChange?.('');
        return;
      }

      // Allow in-progress typing: trailing dot or leading dot
      if (raw === '.' || /^\d+\.$/.test(raw)) {
        setInternalValue(raw);
        setPrecisionError(null);
        onChange?.(raw);
        return;
      }

      if (!/^\d*\.?\d*$/.test(raw)) return;

      // Enforce precision: silently clamp rather than block typing
      const normalized = normalizeDecimalString(raw);
      if (normalized !== null) {
        const dotIdx = normalized.indexOf('.');
        if (dotIdx !== -1) {
          const fracLen = normalized.length - dotIdx - 1;
          if (fracLen > maxDecimals) {
            const clamped = clampToMaxDecimals(normalized, maxDecimals);
            setInternalValue(clamped);
            setPrecisionError(
              `Maximum ${maxDecimals} decimal place${maxDecimals === 1 ? '' : 's'} for this asset.`
            );
            onChange?.(clamped);
            return;
          }
        }
      }

      setPrecisionError(null);
      setInternalValue(raw);
      onChange?.(raw);
    },
    [onChange, maxDecimals]
  );

  const exceedsBalance =
    balance &&
    internalValue &&
    !Number.isNaN(parseFloat(internalValue)) &&
    !Number.isNaN(parseFloat(balance)) &&
    parseFloat(internalValue) > parseFloat(balance);

  return (
    <div className={cn('flex flex-col gap-1.5 w-full', className)}>
      <div className="flex justify-between items-center px-1">
        {label && (
          <label
            htmlFor={inputId}
            className="text-xs font-medium text-muted-foreground uppercase tracking-wider"
          >
            {label}
          </label>
        )}
        {(balance || balanceLoading || balanceError) && (
          <span className="text-xs text-muted-foreground">
            Balance:{' '}
            <span className="font-medium text-foreground/80">
              {balanceLoading
                ? 'Loading...'
                : balanceError
                  ? 'Unavailable'
                  : balance}
            </span>
          </span>
        )}
      </div>

      <div className="relative group">
        <Input
          id={inputId}
          type="text"
          inputMode="decimal"
          placeholder={placeholder}
          value={internalValue}
          onChange={handleChange}
          disabled={disabled}
          readOnly={readOnly}
          aria-invalid={!!(precisionError || exceedsBalance)}
          aria-describedby={
            precisionError || exceedsBalance ? `${inputId}-error` : undefined
          }
          className={cn(
            'h-14 text-2xl font-semibold bg-background/40 border-border/40 focus-visible:ring-primary/30 rounded-xl px-4',
            readOnly &&
              'cursor-default border-transparent bg-transparent px-0 text-3xl',
            !readOnly && 'group-hover:border-primary/20 transition-all shadow-sm',
            (precisionError || exceedsBalance) && 'border-destructive/50'
          )}
        />

        {showMax && onMax && !readOnly && !disabled && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onMax}
            className="absolute right-2 top-1/2 -translate-y-1/2 h-8 px-3 text-xs font-bold text-primary hover:bg-primary/10 hover:text-primary rounded-lg transition-colors"
          >
            MAX
          </Button>
        )}
      </div>

      {(precisionError || exceedsBalance) && (
        <p
          id={`${inputId}-error`}
          role="alert"
          className="text-[10px] font-medium text-destructive px-1 mt-1"
        >
          {precisionError ?? 'Amount exceeds available balance'}
        </p>
      )}

      {showPresets && onPresetSelect && (
        <AmountPresets
          balance={balance || null}
          decimals={maxDecimals}
          onSelect={onPresetSelect}
          disabled={disabled || readOnly}
          className="mt-2"
        />
      )}
    </div>
  );
}
