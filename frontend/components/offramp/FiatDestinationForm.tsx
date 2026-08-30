'use client';

import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { OFFRAMP_FIAT } from '@/lib/offramp/assets';
import { NIGERIAN_BANKS } from '@/lib/offramp/nigerian-banks';
import { cn } from '@/lib/utils';

interface FiatDestinationFormProps {
  bankCode: string;
  accountNumber: string;
  accountName: string;
  onBankCodeChange: (code: string) => void;
  onAccountNumberChange: (value: string) => void;
  onAccountNameChange: (value: string) => void;
  accountNumberError?: string | null;
  className?: string;
}

export function FiatDestinationForm({
  bankCode,
  accountNumber,
  accountName,
  onBankCodeChange,
  onAccountNumberChange,
  onAccountNameChange,
  accountNumberError,
  className,
}: FiatDestinationFormProps) {
  return (
    <div
      className={cn('space-y-4', className)}
      data-testid="offramp-destination-form"
    >
      <div>
        <h2 className="font-display text-lg font-semibold tracking-tight">
          You receive
        </h2>
        <p className="text-sm text-muted-foreground">
          First corridor: {OFFRAMP_FIAT.flag} {OFFRAMP_FIAT.name} to a Nigerian
          bank account.
        </p>
      </div>

      <div className="flex items-center gap-3 rounded-xl border border-border/60 bg-background/40 px-4 py-3">
        <span className="text-2xl" aria-hidden>
          {OFFRAMP_FIAT.flag}
        </span>
        <div>
          <p className="font-semibold text-foreground">
            {OFFRAMP_FIAT.symbol} {OFFRAMP_FIAT.code}
          </p>
          <p className="text-xs text-muted-foreground">{OFFRAMP_FIAT.country}</p>
        </div>
        <span className="ml-auto font-mono text-[10px] font-semibold uppercase tracking-[0.18em] text-primary">
          Live corridor
        </span>
      </div>

      <div className="space-y-2">
        <Label htmlFor="offramp-bank">Bank</Label>
        <Select value={bankCode || undefined} onValueChange={onBankCodeChange}>
          <SelectTrigger
            id="offramp-bank"
            className="w-full"
            data-testid="offramp-bank-select"
          >
            <SelectValue placeholder="Select your bank" />
          </SelectTrigger>
          <SelectContent>
            {NIGERIAN_BANKS.map((bank) => (
              <SelectItem key={bank.code} value={bank.code}>
                {bank.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="offramp-account-number">Account number</Label>
        <Input
          id="offramp-account-number"
          inputMode="numeric"
          autoComplete="off"
          placeholder="10-digit NUBAN"
          value={accountNumber}
          onChange={(e) =>
            onAccountNumberChange(e.target.value.replace(/\D/g, '').slice(0, 10))
          }
          aria-invalid={Boolean(accountNumberError)}
          data-testid="offramp-account-number"
        />
        {accountNumberError ? (
          <p className="text-xs text-destructive" role="alert">
            {accountNumberError}
          </p>
        ) : (
          <p className="text-xs text-muted-foreground">
            Nigerian bank accounts use a 10-digit NUBAN.
          </p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="offramp-account-name">Account name</Label>
        <Input
          id="offramp-account-name"
          autoComplete="name"
          placeholder="Name on the bank account"
          value={accountName}
          onChange={(e) => onAccountNameChange(e.target.value)}
          data-testid="offramp-account-name"
        />
      </div>
    </div>
  );
}
