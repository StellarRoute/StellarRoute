'use client';

import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { normalizeDecimalString } from '@/lib/amount-input';
import { useState, useCallback, useEffect, useId } from 'react';

interface AmountInputProps {
  value: string;
  onChange?: (value: string) => void;
  onMax?: () => void;
  placeholder?: string;
  disabled?: boolean;
  readOnly?: boolean;
  className?: string;
  label?: string;
  balance?: string;
  showMax?: boolean;
  decimals?: number; 
}

export function AmountInput({
  value,
  onChange,
  onMax,
  placeholder = '0.00',
  disabled = false,
  readOnly = false,
  className,
  label,
  balance,
  showMax = true,
  decimals = 7,
}: AmountInputProps) {
  const [internalValue, setInternalValue] = useState(value);
  const inputId = useId();

  useEffect(() => {
    setInternalValue(value);
  }, [value]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      let raw = e.target.value.replace(/,/g, '.'); 
      
      if (raw === '') {
        setInternalValue('');
        onChange?.('');
        return;
      }

      if ((raw.match(/\./g) || []).length > 1) return;

      if (!/^\d*\.?\d*$/.test(raw)) return;

      if (raw.includes('.')) {
        const [, decimalPart] = raw.split('.');
        if (decimalPart && decimalPart.length > (decimals || 7)) {
          return; 
        }
      }

      setInternalValue(raw);
      onChange?.(raw);
    },
    [onChange, decimals]
  );

  return (
    <div className={cn("flex flex-col gap-1.5 w-full", className)}>
      <div className="flex justify-between items-center px-1">
        {label && (
          <label htmlFor={inputId} className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            {label}
          </label>
        )}
        {balance && (
          <span className="text-xs text-muted-foreground">
            Balance: <span className="font-medium text-foreground/80">{balance}</span>
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
          className={cn(
            "h-14 text-2xl font-semibold bg-background/40 border-border/40 focus-visible:ring-primary/30 rounded-xl px-4",
            readOnly && "cursor-default border-transparent bg-transparent px-0 text-3xl",
            !readOnly && "group-hover:border-primary/20 transition-all shadow-sm"
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
      
        {balance && internalValue && parseFloat(internalValue) > parseFloat(balance) && (
        <p className="text-[10px] font-medium text-red-500 px-1 mt-1">
          Amount exceeds available balance
        </p>
      )}

    </div>
  );
}
