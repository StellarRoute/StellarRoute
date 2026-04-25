import { useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Card } from '@/components/ui/card';

export interface SlippageControlProps {
  value: number;
  onChange: (value: number) => void;
  isLoading?: boolean;
  error?: string;
}

export function SlippageControl({ value, onChange, isLoading, error }: SlippageControlProps) {
  const isInvalid = useMemo(() => Number.isNaN(value) || value < 0 || value > 50, [value]);

  if (isLoading) {
    return (
      <Card className="p-3" role="status" aria-busy="true">
        <p className="text-sm text-muted-foreground">Loading slippage control…</p>
      </Card>
    );
  }

  return (
    <Card className="p-3">
      <label className="block text-sm font-medium">Slippage tolerance (%)</label>
      <Input
        type="number"
        step="0.1"
        min={0}
        max={50}
        value={value.toString()}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-1 max-w-[120px]"
      />
      {error && <p className="text-xs text-destructive mt-1">{error}</p>}
      {isInvalid && !error && <p className="text-xs text-warning mt-1">Value must be between 0 and 50%</p>}
    </Card>
  );
}
