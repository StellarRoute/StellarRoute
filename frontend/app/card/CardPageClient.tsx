'use client';

import { useEffect, useState } from 'react';
import { ArrowRight } from 'lucide-react';

import { useWallet } from '@/hooks/useWallet';
import {
  CardBalanceAlert,
  cardBalanceAlertDismissKey,
} from '@/components/card/CardBalanceAlert';
import { CardFace } from '@/components/card/CardFace';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  DEFAULT_CARD_TOP_UP_USDC,
  getCardTopUpPreferences,
} from '@/lib/card/preferences';

type ActivityRow = {
  authorization_id: string;
  state: string;
  merchant: string;
  fiat_amount: number;
  fiat_currency: string;
  amount_stroops: number;
  status: string;
};

const FIXTURE_ROWS: ActivityRow[] = [
  {
    authorization_id: 'auth-1001',
    state: 'approved',
    merchant: 'Café Verde',
    fiat_amount: 4200,
    fiat_currency: 'USD',
    amount_stroops: 42_000_000,
    status: 'active',
  },
  {
    authorization_id: 'auth-1002',
    state: 'captured',
    merchant: 'Northwind Market',
    fiat_amount: 1850,
    fiat_currency: 'USD',
    amount_stroops: 18_500_000,
    status: 'cleared',
  },
];

/**
 * CARD-45 — fields mirror `CardAccount` (lib/card/types.ts) on the wire,
 * in snake_case like every other card payload.
 */
type CardAccountSummary = {
  id: string;
  status: string;
  last4: string;
  expiry: string;
  available_usdc: string;
};

const FIXTURE_ACCOUNT: CardAccountSummary = {
  id: 'card-acct-fixture',
  status: 'active',
  last4: '4567',
  expiry: '12/28',
  available_usdc: '18.40',
};

const USDC_INPUT_PATTERN = /^\d+(\.\d{1,7})?$/;

function formatUsdc(value: number): string {
  return value.toFixed(2);
}

function parseUsdcAmount(raw: string): number | null {
  const trimmed = raw.trim();
  if (!USDC_INPUT_PATTERN.test(trimmed)) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) && value > 0 ? value : null;
}

export function CardPageClient() {
  const { session } = useWallet();
  const [isFrozen, setIsFrozen] = useState(false);
  const [rows, setRows] = useState<ActivityRow[]>(FIXTURE_ROWS);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [account, setAccount] = useState<CardAccountSummary | null>(FIXTURE_ACCOUNT);
  const [topUpOpen, setTopUpOpen] = useState(false);
  const [topUpAmount, setTopUpAmount] = useState('');
  const [topUpError, setTopUpError] = useState<string | null>(null);
  const canUnfreeze = Boolean(session.isConnected);

  useEffect(() => {
    let cancelled = false;

    async function loadActivity() {
      try {
        const response = await fetch('/api/v1/card/authorizations');
        if (!response.ok) {
          if (response.status === 404) {
            if (!cancelled) setError('Card program is not enabled.');
            return;
          }
          if (!cancelled) setError('Unable to load card activity.');
          return;
        }

        const payload = (await response.json()) as ActivityRow[];
        if (!cancelled) {
          setRows(payload.length > 0 ? payload : []);
          setError(null);
        }
      } catch {
        if (!cancelled) {
          setRows(FIXTURE_ROWS);
          setError(null);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    void loadActivity();

    return () => {
      cancelled = true;
    };
  }, []);

  // CARD-45: the alert needs the cardholder's available USDC. The endpoint is
  // gated with the card program, so a 404/invalid payload keeps the demo
  // account — the same degradation the activity list already uses.
  useEffect(() => {
    let cancelled = false;

    async function loadAccount() {
      try {
        const response = await fetch('/api/v1/card/accounts');
        if (!response.ok) return;

        const payload = (await response.json()) as {
          accounts?: CardAccountSummary[];
        } | CardAccountSummary[];
        const accounts = Array.isArray(payload) ? payload : payload.accounts;
        const first = accounts?.[0];
        if (!cancelled && first && parseUsdcAmount(first.available_usdc) !== null) {
          setAccount(first);
        }
      } catch {
        // Keep the current account; a missing backend never breaks the page.
      }
    }

    void loadAccount();

    return () => {
      cancelled = true;
    };
  }, []);

  // Preferences are per-address and browser-only (CARD-52); defaults apply
  // when disconnected or unset.
  const { alertThresholdUsdc, recommendedTopUpUsdc } = (() => {
    if (!session.address) {
      return {
        alertThresholdUsdc: DEFAULT_CARD_ALERT_THRESHOLD_USDC,
        recommendedTopUpUsdc: DEFAULT_CARD_TOP_UP_USDC,
      };
    }
    const preferences = getCardTopUpPreferences(session.address);
    return {
      alertThresholdUsdc: preferences.alertThresholdUsdc,
      recommendedTopUpUsdc: preferences.defaultTopUpUsdc,
    };
  })();

  const availableUsdc = account
    ? Number.parseFloat(account.available_usdc)
    : null;

  const handleTopUp = (recommendedUsdc: number) => {
    setTopUpAmount(formatUsdc(recommendedUsdc));
    setTopUpError(null);
    setTopUpOpen(true);
  };

  const handleConfirmTopUp = () => {
    const amount = parseUsdcAmount(topUpAmount);
    if (amount === null) {
      setTopUpError('Enter a top-up amount greater than 0 USDC.');
      return;
    }
    setTopUpOpen(false);
  };

  const handleFreeze = () => {
    if (typeof window !== 'undefined' && !window.confirm('Freeze this card program? New authorizations will be declined immediately.')) {
      return;
    }
    setIsFrozen(true);
  };

  const handleUnfreeze = () => {
    if (!canUnfreeze) return;
    if (typeof window !== 'undefined' && !window.confirm('Unfreeze this card program after wallet verification?')) {
      return;
    }
    setIsFrozen(false);
  };

  return (
    <main className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-12">
      <section
        className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950"
        data-testid="card-balance-section"
      >
        <h2 className="text-lg font-semibold text-slate-900 dark:text-slate-50">
          Your card
        </h2>
        <div className="mt-4 flex flex-col gap-4">
          <CardFace
            last4={account?.last4 ?? '0000'}
            expiry={account?.expiry ?? '--/--'}
            usdcAvailable={formatUsdc(availableUsdc ?? 0)}
            isFrozen={isFrozen}
          />
          <CardBalanceAlert
            availableUsdc={availableUsdc}
            thresholdUsdc={alertThresholdUsdc}
            recommendedUsdc={recommendedTopUpUsdc}
            dismissKey={cardBalanceAlertDismissKey(
              session.address ?? null,
              alertThresholdUsdc,
            )}
            onTopUp={handleTopUp}
          />
        </div>
      </section>

      <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <p className="text-sm font-medium uppercase tracking-[0.2em] text-slate-500">
          Card Controls
        </p>
        <h1 className="mt-3 text-3xl font-semibold text-slate-900 dark:text-slate-50">
          {isFrozen ? 'Card program frozen' : 'Card program active'}
        </h1>
        <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">
          Freeze stops new authorizations immediately. Unfreeze requires a connected wallet and a second confirmation.
        </p>

        <div className="mt-6 flex flex-wrap gap-3">
          <button
            type="button"
            onClick={handleFreeze}
            className="inline-flex items-center justify-center rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-700 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-slate-300"
          >
            Freeze
          </button>
          <button
            type="button"
            onClick={handleUnfreeze}
            disabled={!canUnfreeze}
            className="inline-flex items-center justify-center rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:border-slate-400 hover:text-slate-900 disabled:cursor-not-allowed disabled:border-slate-200 disabled:text-slate-400 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:border-slate-500 dark:hover:text-slate-100 disabled:dark:border-slate-800 disabled:dark:text-slate-500"
          >
            Unfreeze
          </button>
        </div>

        <div className="mt-6 rounded-xl border border-slate-200 bg-slate-50 p-4 text-sm text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
          {canUnfreeze
            ? 'Wallet connected. Unfreeze can be used at any time.'
            : 'Connect a wallet to enable Unfreeze.'}
        </div>
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div className="mb-4 flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-slate-900 dark:text-slate-50">Activity</h2>
          <span className="text-xs uppercase tracking-[0.2em] text-slate-500">{rows.length} items</span>
        </div>

        {loading ? (
          <div className="rounded-xl border border-dashed border-slate-200 p-6 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
            Loading card activity…
          </div>
        ) : error ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
            {error}
          </div>
        ) : rows.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-200 p-8 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
            No card activity yet.
          </div>
        ) : (
          <div className="overflow-hidden rounded-xl border border-slate-200 dark:border-slate-800">
            <div className="grid grid-cols-[1.4fr_1fr_1fr_0.8fr] gap-3 bg-slate-50 px-4 py-3 text-xs font-medium uppercase tracking-[0.12em] text-slate-500 dark:bg-slate-900 dark:text-slate-400">
              <span>Merchant</span>
              <span>Fiat</span>
              <span>USDC</span>
              <span>Status</span>
            </div>
            {rows.map((row) => (
              <div
                key={row.authorization_id}
                className="grid grid-cols-[1.4fr_1fr_1fr_0.8fr] gap-3 border-t border-slate-200 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:text-slate-200"
              >
                <div>
                  <div className="font-medium text-slate-900 dark:text-slate-50">{row.merchant}</div>
                  <div className="text-xs text-slate-500">#{row.authorization_id}</div>
                </div>
                <div>
                  {row.fiat_amount / 100} {row.fiat_currency}
                </div>
                <div>{(row.amount_stroops / 10_000_000).toFixed(2)} USDC</div>
                <div className="capitalize">{row.status}</div>
              </div>
            ))}
          </div>
        )}
      </section>

      <CardTopUpModal
        open={topUpOpen}
        amount={topUpAmount}
        error={topUpError}
        onAmountChange={(raw) => {
          setTopUpAmount(raw);
          setTopUpError(null);
        }}
        onOpenChange={setTopUpOpen}
        onConfirm={handleConfirmTopUp}
      />
    </main>
  );
}

/**
 * CARD-45's 1-click bridge top-up trigger. CARD-41's CardTopUpModal is not
 * vendored in this tree yet, so the funding entry point lives here: the
 * amount field arrives pre-filled from the alert (default 50.00 USDC) and
 * the bridge CTA carries the amount to the existing cross-chain funding
 * page. Nothing is signed or executed on this page.
 */
function CardTopUpModal({
  open,
  amount,
  error,
  onAmountChange,
  onOpenChange,
  onConfirm,
}: {
  open: boolean;
  amount: string;
  error: string | null;
  onAmountChange: (raw: string) => void;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  const parsed = parseUsdcAmount(amount);
  const bridgeHref = parsed
    ? `/cross-chain-swap?amount=${encodeURIComponent(String(parsed))}&destination=card`
    : '/cross-chain-swap?destination=card';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex w-[min(100%,90vw)] max-h-[min(90dvh,90vh)] flex-col gap-0 overflow-hidden p-0 rounded-2xl border-border/40 bg-background/95 backdrop-blur-xl sm:max-w-[480px] sm:rounded-[28px]"
        data-testid="card-top-up-modal"
      >
        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto overscroll-contain p-5 sm:p-7">
          <DialogHeader className="space-y-2 text-left">
            <DialogTitle className="font-display text-2xl font-bold tracking-tight">
              Bridge top-up
            </DialogTitle>
            <DialogDescription className="text-pretty text-muted-foreground">
              Fund your card from another chain. The amount below was chosen
              to clear the low-balance alert; edit it if you want to top up
              more, then continue in the bridge flow.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-2 rounded-xl border border-border/30 bg-background/50 p-4">
            <Label htmlFor="card-top-up-amount" className="text-sm font-semibold">
              Top-up amount (USDC)
            </Label>
            <Input
              id="card-top-up-amount"
              type="text"
              inputMode="decimal"
              value={amount}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? 'card-top-up-amount-error' : undefined}
              onChange={(event) => onAmountChange(event.target.value)}
              data-testid="card-top-up-amount-input"
            />
            {error && (
              <p
                id="card-top-up-amount-error"
                role="alert"
                className="text-xs font-medium text-destructive"
                data-testid="card-top-up-amount-error"
              >
                {error}
              </p>
            )}
          </div>
        </div>

        <DialogFooter className="shrink-0 flex-col gap-2 border-t border-border/20 bg-muted/10 p-4 sm:flex-col sm:p-6">
          <Button
            type="button"
            className="min-h-11 w-full font-semibold"
            onClick={onConfirm}
            data-testid="card-top-up-confirm"
          >
            Confirm top-up amount
          </Button>
          <a
            href={bridgeHref}
            className="inline-flex min-h-11 w-full items-center justify-center gap-2 rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:border-slate-400 hover:text-slate-900 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:border-slate-500 dark:hover:text-slate-100"
            data-testid="card-top-up-bridge-link"
          >
            Continue to bridge
            <ArrowRight className="h-4 w-4" aria-hidden />
          </a>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
