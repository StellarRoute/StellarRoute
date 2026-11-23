'use client';

import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown, Search } from 'lucide-react';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { OFFRAMP_FIAT } from '@/lib/offramp/assets';
import { NIGERIAN_BANKS } from '@/lib/offramp/nigerian-banks';
import { cn } from '@/lib/utils';

/** Debounce delay in ms for the institution search input. */
const SEARCH_DEBOUNCE_MS = 300;

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

/**
 * Searchable bank combobox with debounced filtering.
 *
 * Replaces the plain `<Select>` with an accessible input + listbox pattern so
 * users can type to filter the institution list rather than scrolling through
 * all Nigerian banks. The `query` state changes immediately on every keystroke
 * (keeping the input responsive); the filtered list only updates after the
 * debounce settles, preventing excessive re-renders on fast typing.
 */
function BankSearchCombobox({
  value,
  onChange,
}: {
  value: string;
  onChange: (code: string) => void;
}) {
  const selectedBank = NIGERIAN_BANKS.find((b) => b.code === value) ?? null;

  // Raw input value — updates on every keystroke.
  const [inputValue, setInputValue] = useState(selectedBank?.name ?? '');
  const [isOpen, setIsOpen] = useState(false);

  // Debounced query used to filter the list.
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const containerRef = useRef<HTMLDivElement>(null);

  // Keep input display in sync when parent resets bankCode externally.
  useEffect(() => {
    const name = NIGERIAN_BANKS.find((b) => b.code === value)?.name ?? '';
    setInputValue(name);
  }, [value]);

  // Schedule debounced query update on every input change.
  const handleInputChange = (raw: string) => {
    setInputValue(raw);
    setIsOpen(true);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setDebouncedQuery(raw);
    }, SEARCH_DEBOUNCE_MS);
  };

  // Cleanup pending debounce on unmount.
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  // Close dropdown on outside click.
  useEffect(() => {
    const handlePointerDown = (e: PointerEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
        // Restore display text to the confirmed selection on blur-away.
        setInputValue(selectedBank?.name ?? '');
      }
    };
    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, [selectedBank]);

  const filteredBanks =
    debouncedQuery.trim() === ''
      ? NIGERIAN_BANKS
      : NIGERIAN_BANKS.filter((b) =>
          b.name.toLowerCase().includes(debouncedQuery.toLowerCase()) ||
          b.code.includes(debouncedQuery.trim())
        );

  const handleSelect = (code: string, name: string) => {
    onChange(code);
    setInputValue(name);
    setDebouncedQuery('');
    setIsOpen(false);
  };

  return (
    <div ref={containerRef} className="relative">
      <div className="relative flex items-center">
        <Search
          className="pointer-events-none absolute left-3 h-4 w-4 text-muted-foreground"
          aria-hidden
        />
        <input
          id="offramp-bank"
          role="combobox"
          aria-expanded={isOpen}
          aria-autocomplete="list"
          aria-controls="offramp-bank-listbox"
          aria-label="Search for your bank"
          autoComplete="off"
          placeholder="Search bank name or code…"
          value={inputValue}
          onChange={(e) => handleInputChange(e.target.value)}
          onFocus={() => setIsOpen(true)}
          className={cn(
            'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2',
            'pl-9 pr-9 text-sm ring-offset-background',
            'placeholder:text-muted-foreground',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
            'disabled:cursor-not-allowed disabled:opacity-50',
          )}
          data-testid="offramp-bank-search"
        />
        <ChevronDown
          className="pointer-events-none absolute right-3 h-4 w-4 text-muted-foreground"
          aria-hidden
        />
      </div>

      {isOpen && (
        <ul
          id="offramp-bank-listbox"
          role="listbox"
          aria-label="Bank options"
          className={cn(
            'absolute z-50 mt-1 max-h-56 w-full overflow-auto',
            'rounded-md border border-border bg-popover shadow-md',
          )}
          data-testid="offramp-bank-listbox"
        >
          {filteredBanks.length === 0 ? (
            <li className="px-3 py-2 text-sm text-muted-foreground">
              No banks match &ldquo;{debouncedQuery}&rdquo;
            </li>
          ) : (
            filteredBanks.map((bank) => (
              <li
                key={bank.code}
                role="option"
                aria-selected={bank.code === value}
                onClick={() => handleSelect(bank.code, bank.name)}
                className={cn(
                  'flex cursor-pointer items-center justify-between px-3 py-2 text-sm',
                  'hover:bg-accent hover:text-accent-foreground',
                  bank.code === value && 'bg-accent/50 font-medium',
                )}
                data-testid={`offramp-bank-option-${bank.code}`}
              >
                <span>{bank.name}</span>
                <span className="font-mono text-xs text-muted-foreground">
                  {bank.code}
                  {bank.code === value && (
                    <Check className="ml-1 inline h-3 w-3 text-primary" aria-hidden />
                  )}
                </span>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
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
        <BankSearchCombobox value={bankCode} onChange={onBankCodeChange} />
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
