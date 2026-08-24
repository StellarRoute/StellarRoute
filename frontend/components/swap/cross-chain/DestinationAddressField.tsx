'use client';

import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { ChainDefinition } from '@/lib/cross-chain/types';
import type { RecipientValidationResult } from '@/lib/cross-chain/types';

interface DestinationAddressFieldProps {
  chain: ChainDefinition;
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
  value: string;
  onChange: (value: string) => void;
  validation: RecipientValidationResult;
  disabled?: boolean;
  setupHint?: string | null;
}

export function DestinationAddressField({
  chain,
  enabled,
  onEnabledChange,
  value,
  onChange,
  validation,
  disabled = false,
  setupHint,
}: DestinationAddressFieldProps) {
  return (
    <div className="space-y-3 rounded-2xl border border-border/40 bg-muted/20 p-4">
      <div className="flex items-start gap-3">
        <Checkbox
          id="recipient-override"
          checked={enabled}
          onCheckedChange={(checked) => onEnabledChange(checked === true)}
          className="mt-1 min-h-11 min-w-11"
          aria-label="Use custom destination recipient"
          disabled={disabled}
        />
        <div className="space-y-1 min-w-0 flex-1">
          <Label htmlFor="recipient-override" className="text-sm font-semibold">
            Destination recipient
          </Label>
          <p className="text-xs text-muted-foreground">
            Override the default wallet address on {chain.label}. Required format
            must match the destination chain.
          </p>
          {!enabled && setupHint && (
            <p
              className="text-xs text-muted-foreground"
              role="status"
              data-testid="destination-recipient-setup-hint"
            >
              {setupHint}
            </p>
          )}
        </div>
      </div>

      {enabled && (
        <div className="space-y-1.5">
          <Label htmlFor="recipient-address" className="text-xs text-muted-foreground">
            {chain.label} address
          </Label>
          <Input
            id="recipient-address"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={`${chain.shortLabel} recipient`}
            className="min-h-11 font-mono text-sm"
            aria-invalid={!validation.valid}
            data-testid="destination-recipient-input"
            disabled={disabled}
          />
          {!validation.valid && (
            <p role="alert" className="text-xs text-destructive">
              {validation.message}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
