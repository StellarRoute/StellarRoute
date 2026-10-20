'use client';

import { useCallback, useEffect, useState } from 'react';
import { Settings } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import {
  CARD_TOP_UP_PRESETS,
  DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  DEFAULT_CARD_TOP_UP_USDC,
  defaultCardTopUpPreferences,
  getCardTopUpPreferences,
  saveCardTopUpPreferences,
  type CardTopUpPreferences,
} from '@/lib/card/preferences';

export interface CardTopUpPreferencesModalProps {
  /** Active Stellar public key; preferences are stored per address. */
  stellarAddress: string | null;
  onPreferencesChange?: (preferences: CardTopUpPreferences) => void;
  className?: string;
}

type PreferenceField = 'alertThresholdUsdc' | 'defaultTopUpUsdc';

function parseUsdcAmount(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return null;

  const value = Number(trimmed);
  return Number.isFinite(value) && value > 0 ? value : null;
}

function formatUsdc(value: number): string {
  return value.toFixed(2);
}

function fieldLabel(field: PreferenceField): string {
  return field === 'alertThresholdUsdc'
    ? 'Low balance alert threshold'
    : 'Default top-up amount';
}

interface AmountFieldProps {
  field: PreferenceField;
  description: string;
  value: string;
  error: string | null;
  disabled: boolean;
  onChange: (raw: string) => void;
  onPreset: (amount: number) => void;
}

function AmountField({
  field,
  description,
  value,
  error,
  disabled,
  onChange,
  onPreset,
}: AmountFieldProps) {
  const inputId = `card-preference-${field}`;

  return (
    <div className="space-y-3 rounded-xl border border-border/30 bg-background/50 p-4">
      <Label htmlFor={inputId} className="text-sm font-semibold">
        {fieldLabel(field)}
      </Label>
      <p className="text-xs text-muted-foreground">{description}</p>

      <div className="flex flex-wrap gap-2">
        {CARD_TOP_UP_PRESETS.map((preset) => (
          <Button
            key={preset}
            type="button"
            variant={
              parseUsdcAmount(value) === preset ? 'default' : 'outline'
            }
            size="sm"
            disabled={disabled}
            onClick={() => onPreset(preset)}
            data-testid={`card-preference-preset-${field}-${preset}`}
          >
            {preset} USDC
          </Button>
        ))}
      </div>

      <Input
        id={inputId}
        type="text"
        inputMode="decimal"
        value={value}
        disabled={disabled}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? `${inputId}-error` : undefined}
        onChange={(event) => onChange(event.target.value)}
        data-testid={`card-preference-input-${field}`}
      />

      {error && (
        <p
          id={`${inputId}-error`}
          role="alert"
          className="text-xs font-medium text-destructive"
          data-testid={`card-preference-error-${field}`}
        >
          {error}
        </p>
      )}
    </div>
  );
}

/**
 * CARD-52 - Local threshold and auto-top-up preferences.
 *
 * Gear trigger for the card controls header plus a dialog that stores the
 * alert threshold and default top-up amount in localStorage under the active
 * Stellar public key. Presets persist on click; custom amounts persist on
 * Save. CardBalanceAlert reads `resolveAlertThresholdUsdc` and CardTopUpModal
 * pre-fills from `getPreferredTopUpAmount`, so saving here takes effect
 * immediately. Nothing is sent to a backend.
 */
export function CardTopUpPreferencesModal({
  stellarAddress,
  onPreferencesChange,
  className,
}: CardTopUpPreferencesModalProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [saved, setSaved] = useState<CardTopUpPreferences>({
    alertThresholdUsdc: DEFAULT_CARD_ALERT_THRESHOLD_USDC,
    defaultTopUpUsdc: DEFAULT_CARD_TOP_UP_USDC,
  });
  const [thresholdRaw, setThresholdRaw] = useState(
    formatUsdc(DEFAULT_CARD_ALERT_THRESHOLD_USDC),
  );
  const [topUpRaw, setTopUpRaw] = useState(formatUsdc(DEFAULT_CARD_TOP_UP_USDC));
  const [errors, setErrors] = useState<Partial<Record<PreferenceField, string>>>({});

  useEffect(() => {
    const current = stellarAddress
      ? getCardTopUpPreferences(stellarAddress)
      : defaultCardTopUpPreferences();

    setSaved(current);
    setThresholdRaw(formatUsdc(current.alertThresholdUsdc));
    setTopUpRaw(formatUsdc(current.defaultTopUpUsdc));
    setErrors({});
  }, [stellarAddress, isOpen]);

  const persist = useCallback(
    (next: CardTopUpPreferences) => {
      if (!stellarAddress) return;

      const stored = saveCardTopUpPreferences(stellarAddress, next);
      setSaved(stored);
      setThresholdRaw(formatUsdc(stored.alertThresholdUsdc));
      setTopUpRaw(formatUsdc(stored.defaultTopUpUsdc));
      onPreferencesChange?.(stored);
    },
    [stellarAddress, onPreferencesChange],
  );

  const handlePreset = (field: PreferenceField, amount: number) => {
    const otherField: PreferenceField =
      field === 'alertThresholdUsdc' ? 'defaultTopUpUsdc' : 'alertThresholdUsdc';
    const otherRaw = field === 'alertThresholdUsdc' ? topUpRaw : thresholdRaw;
    const other = parseUsdcAmount(otherRaw) ?? saved[otherField];

    setErrors({});
    persist(
      field === 'alertThresholdUsdc'
        ? { alertThresholdUsdc: amount, defaultTopUpUsdc: other }
        : { alertThresholdUsdc: other, defaultTopUpUsdc: amount },
    );
  };

  const handleSave = () => {
    const alertThresholdUsdc = parseUsdcAmount(thresholdRaw);
    const defaultTopUpUsdc = parseUsdcAmount(topUpRaw);
    const nextErrors: Partial<Record<PreferenceField, string>> = {};

    if (alertThresholdUsdc === null) {
      nextErrors.alertThresholdUsdc =
        'Enter an alert threshold greater than 0 USDC.';
    }
    if (defaultTopUpUsdc === null) {
      nextErrors.defaultTopUpUsdc = 'Enter a top-up amount greater than 0 USDC.';
    }

    setErrors(nextErrors);
    if (alertThresholdUsdc === null || defaultTopUpUsdc === null) return;

    persist({ alertThresholdUsdc, defaultTopUpUsdc });
    setIsOpen(false);
  };

  const handleReset = () => {
    setErrors({});
    persist({
      alertThresholdUsdc: DEFAULT_CARD_ALERT_THRESHOLD_USDC,
      defaultTopUpUsdc: DEFAULT_CARD_TOP_UP_USDC,
    });
  };

  const isCustomDisabled = !stellarAddress;

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Card top-up preferences"
          data-testid="card-preferences-gear"
        >
          <Settings className="h-4 w-4" aria-hidden />
        </Button>
      </DialogTrigger>

      <DialogContent
        className={cn(
          'flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-0',
          'rounded-2xl border-border/40 bg-background/95 backdrop-blur-xl sm:max-w-[480px] sm:rounded-[28px]',
          className,
        )}
        data-testid="card-top-up-preferences-modal"
      >
        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto overscroll-contain p-5 sm:p-7">
          <DialogHeader className="space-y-2 text-left">
            <DialogTitle className="font-display text-2xl font-bold tracking-tight">
              Top-up preferences
            </DialogTitle>
            <DialogDescription className="text-pretty text-muted-foreground">
              Choose when the balance warning fires and how much a top-up
              pre-fills. Saved in this browser for {stellarAddress ?? 'the next wallet you connect'}.
            </DialogDescription>
          </DialogHeader>

          {isCustomDisabled && (
            <p
              role="status"
              className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200"
              data-testid="card-preferences-disconnected"
            >
              Connect a Stellar wallet to save preferences for your address.
            </p>
          )}

          <AmountField
            field="alertThresholdUsdc"
            description={`Warn when available USDC drops below this. Default ${DEFAULT_CARD_ALERT_THRESHOLD_USDC} USDC.`}
            value={thresholdRaw}
            error={errors.alertThresholdUsdc ?? null}
            disabled={isCustomDisabled}
            onChange={setThresholdRaw}
            onPreset={(amount) => handlePreset('alertThresholdUsdc', amount)}
          />

          <AmountField
            field="defaultTopUpUsdc"
            description={`Pre-fills the top-up amount. Default ${DEFAULT_CARD_TOP_UP_USDC} USDC.`}
            value={topUpRaw}
            error={errors.defaultTopUpUsdc ?? null}
            disabled={isCustomDisabled}
            onChange={setTopUpRaw}
            onPreset={(amount) => handlePreset('defaultTopUpUsdc', amount)}
          />
        </div>

        <DialogFooter className="shrink-0 flex-col gap-2 border-t border-border/20 bg-muted/10 p-4 sm:flex-col sm:p-6">
          <Button
            className="min-h-11 w-full font-semibold"
            disabled={isCustomDisabled}
            onClick={handleSave}
            data-testid="card-preferences-save"
          >
            Save preferences
          </Button>
          <Button
            type="button"
            variant="outline"
            className="min-h-11 w-full"
            disabled={isCustomDisabled}
            onClick={handleReset}
            data-testid="card-preferences-reset"
          >
            Reset to Defaults
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
