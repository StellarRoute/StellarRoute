'use client';

import { useMemo, useState } from 'react';
import { ArrowDown, CheckCircle2, Info } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  DEFAULT_OFFRAMP_SOURCE_ID,
  OFFRAMP_FIAT,
  OFFRAMP_SOURCE_ASSETS,
  buildOfframpQuotePreview,
  buildOfframpRouteSteps,
  findOfframpSource,
  isValidNigerianAccountNumber,
  resolveOfframpMode,
} from '@/lib/offramp';
import type { OfframpMode } from '@/lib/offramp/types';
import { cn } from '@/lib/utils';
import { OfframpModeToggle } from './OfframpModeToggle';
import { SourceAssetPicker } from './SourceAssetPicker';
import { OfframpRouteRail } from './OfframpRouteRail';
import { FiatDestinationForm } from './FiatDestinationForm';
import { OfframpQuoteSummary } from './OfframpQuoteSummary';

type SubmitState = 'idle' | 'ready' | 'blocked_coming_soon';

export function OfframpDashboard() {
  const [mode, setMode] = useState<OfframpMode>('direct');
  const [sourceId, setSourceId] = useState(DEFAULT_OFFRAMP_SOURCE_ID);
  const [amount, setAmount] = useState('');
  const [bankCode, setBankCode] = useState('');
  const [accountNumber, setAccountNumber] = useState('');
  const [accountName, setAccountName] = useState('');
  const [touchedAccount, setTouchedAccount] = useState(false);
  const [submitState, setSubmitState] = useState<SubmitState>('idle');

  const asset =
    findOfframpSource(sourceId) ??
    findOfframpSource(DEFAULT_OFFRAMP_SOURCE_ID)!;

  const effectiveMode = resolveOfframpMode(asset);
  const displayMode = mode === 'direct' ? 'direct' : effectiveMode;

  const quote = useMemo(
    () =>
      buildOfframpQuotePreview({
        asset,
        amount,
        mode: displayMode,
      }),
    [asset, amount, displayMode],
  );

  const routeSteps = useMemo(
    () => buildOfframpRouteSteps(asset, displayMode),
    [asset, displayMode],
  );

  const accountNumberError =
    touchedAccount && accountNumber.length > 0 && !isValidNigerianAccountNumber(accountNumber)
      ? 'Enter a valid 10-digit NUBAN account number.'
      : null;

  const canSubmit =
    Boolean(quote) &&
    Boolean(bankCode) &&
    isValidNigerianAccountNumber(accountNumber) &&
    accountName.trim().length >= 2 &&
    asset.status !== 'coming_soon';

  function handleModeChange(next: OfframpMode) {
    setMode(next);
    setSubmitState('idle');
    if (next === 'direct') {
      setSourceId(DEFAULT_OFFRAMP_SOURCE_ID);
    }
  }

  function handleSourceSelect(id: string) {
    setSourceId(id);
    setSubmitState('idle');
    const next = findOfframpSource(id);
    if (next && !next.isStellarUsdc) {
      setMode('bridge');
    }
  }

  function handleContinue() {
    setTouchedAccount(true);
    if (!canSubmit) return;
    if (asset.status === 'coming_soon') {
      setSubmitState('blocked_coming_soon');
      return;
    }
    setSubmitState('ready');
  }

  return (
    <div className="offramp-dashboard space-y-10" data-testid="offramp-dashboard">
      <header className="offramp-hero relative overflow-hidden rounded-[1.75rem] border border-border/50 px-6 py-8 sm:px-10 sm:py-10">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_80%_70%_at_0%_0%,color-mix(in_srgb,var(--primary)_18%,transparent),transparent_55%),radial-gradient(ellipse_60%_50%_at_100%_20%,color-mix(in_srgb,#008751_14%,transparent),transparent_50%)]"
        />
        <div
          aria-hidden
          className="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-transparent via-primary/40 to-transparent"
        />
        <div className="relative max-w-2xl space-y-4">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.28em] text-primary">
            Cash corridor · {OFFRAMP_FIAT.flag} Naira first
          </p>
          <h1 className="font-display text-3xl font-bold tracking-tight text-foreground sm:text-4xl lg:text-[2.75rem] lg:leading-[1.1]">
            Stablecoin to local fiat
          </h1>
          <p className="max-w-xl text-base leading-relaxed text-muted-foreground sm:text-lg">
            Move USDC (or bridge another coin into Stellar USDC) and cash out to
            Nigerian Naira. Non-custodial on-chain legs; bank payout via partner
            rails when settlement goes live.
          </p>
        </div>
      </header>

      <OfframpModeToggle mode={mode} onChange={handleModeChange} />

      <div className="grid gap-8 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)] lg:gap-10">
        <section className="space-y-8" aria-label="Offramp form">
          <SourceAssetPicker
            assets={OFFRAMP_SOURCE_ASSETS}
            selectedId={sourceId}
            onSelect={handleSourceSelect}
            directOnly={mode === 'direct'}
          />

          <div className="space-y-2">
            <Label htmlFor="offramp-amount">Amount</Label>
            <div className="relative">
              <Input
                id="offramp-amount"
                inputMode="decimal"
                placeholder="0.00"
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value);
                  setSubmitState('idle');
                }}
                className="h-12 pr-20 font-mono text-lg"
                data-testid="offramp-amount"
              />
              <span className="pointer-events-none absolute inset-y-0 right-3 flex items-center font-mono text-sm font-semibold text-muted-foreground">
                {asset.symbol}
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              On {asset.chainLabel}
              {displayMode === 'bridge' &&
              !asset.isStellarUsdc &&
              asset.kind !== 'stellar_xlm' &&
              asset.kind !== 'stellar_usdc'
                ? ' · will bridge into Stellar USDC before payout'
                : null}
              {asset.status === 'swap_then_offramp'
                ? ' · swap to USDC on Stellar, then cash out'
                : null}
            </p>
          </div>

          <div className="flex justify-center" aria-hidden>
            <span className="flex size-10 items-center justify-center rounded-full border border-border/70 bg-background text-primary shadow-sm">
              <ArrowDown className="size-4" />
            </span>
          </div>

          <FiatDestinationForm
            bankCode={bankCode}
            accountNumber={accountNumber}
            accountName={accountName}
            onBankCodeChange={(code) => {
              setBankCode(code);
              setSubmitState('idle');
            }}
            onAccountNumberChange={(value) => {
              setAccountNumber(value);
              setTouchedAccount(true);
              setSubmitState('idle');
            }}
            onAccountNameChange={(value) => {
              setAccountName(value);
              setSubmitState('idle');
            }}
            accountNumberError={accountNumberError}
          />

          <div className="space-y-3">
            <Button
              size="lg"
              className="h-12 w-full text-base"
              disabled={!canSubmit}
              onClick={handleContinue}
              data-testid="offramp-continue"
            >
              {displayMode === 'direct'
                ? 'Preview Naira payout'
                : 'Preview bridge + Naira payout'}
            </Button>
            <p className="flex items-start gap-2 text-xs leading-relaxed text-muted-foreground">
              <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
              Bank credits are not live yet. This dashboard locks your route and
              quote shape so partner settlement can plug in without redesigning
              the flow.
            </p>
          </div>

          {submitState === 'ready' && quote && (
            <div
              className="animate-in fade-in slide-in-from-bottom-2 rounded-2xl border border-success/30 bg-success/10 px-5 py-4 duration-300"
              role="status"
              data-testid="offramp-ready-banner"
            >
              <div className="flex items-start gap-3">
                <CheckCircle2 className="mt-0.5 size-5 text-success" />
                <div>
                  <p className="font-semibold text-foreground">
                    Route ready · ₦{quote.receiveNgn} indicative
                  </p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    {displayMode === 'direct'
                      ? 'Stellar USDC → Nigerian bank.'
                      : `${asset.symbol} on ${asset.chainLabel} → bridge to Stellar USDC → Nigerian bank.`}{' '}
                    Connect a wallet and complete the on-chain leg when payout
                    partners are enabled on this deployment.
                  </p>
                </div>
              </div>
            </div>
          )}
        </section>

        <aside className="space-y-6 lg:sticky lg:top-24 lg:self-start">
          <div
            className={cn(
              'rounded-[1.5rem] border border-border/60 bg-card/50 p-5 sm:p-6',
              'shadow-[0_1px_0_color-mix(in_srgb,var(--foreground)_4%,transparent)]',
            )}
          >
            <p className="mb-4 font-mono text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              How it moves
            </p>
            <OfframpRouteRail steps={routeSteps} />
          </div>

          <OfframpQuoteSummary quote={quote} />
        </aside>
      </div>
    </div>
  );
}
