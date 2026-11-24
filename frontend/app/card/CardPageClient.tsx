'use client';

import { useCallback, useEffect, useState } from 'react';

import { CardFxFreshnessBadge } from '@/components/card/CardFxFreshnessBadge';
import {
  computeCardFxQuotes,
  formatCardFxRate,
  getCardFxQuote,
  type CardFxSnapshot,
} from '@/lib/card/fx';
import { useWallet } from '@/hooks/useWallet';
import {
  CardBalanceAlert,
  cardBalanceAlertDismissKey,
} from '@/components/card/CardBalanceAlert';
import { CardTopUpModal } from '@/components/card/CardTopUpModal';
import {
  DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  DEFAULT_CARD_TOP_UP_USDC,
  getCardTopUpPreferences,
} from '@/lib/card/preferences';

import { useCardFreezeAction } from '@/hooks/useCardFreezeAction';
import { CardFreezeButton } from '@/components/card/CardFreezeButton';
import { CardFace } from '@/components/card/CardFace';
import { CardNicknameEditor } from '@/components/card/CardNicknameEditor';

import { calculateDailySpendingProgress } from '@/lib/card/limits';
import { CardDailyLimitProgress } from '@/components/card/CardDailyLimitProgress';

// The card program's offramp corridor quotes fiat per USDC, so NGN leads.
const FEATURED_FIAT_CURRENCY = 'NGN';


type ActivityRow = {
  authorization_id: string;
  type: string;
  state: string;
  merchant: string;
  fiat_amount: number;
  fiat_currency: string;
  amount_stroops: number;
  status: string;
  settledAt: string;
};

/** Default daily limit in USDC ($500.00). */
const DAILY_LIMIT_USDC = 500;

const FIXTURE_ROWS: ActivityRow[] = [
  {
    authorization_id: 'auth-1001',
    type: 'pos_debit',
    state: 'settled',
    merchant: 'Café Verde',
    fiat_amount: 4200,
    fiat_currency: 'USD',
    amount_stroops: 42_000_000,
    status: 'active',
    settledAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
  },
  {
    authorization_id: 'auth-1002',
    type: 'pos_debit',
    state: 'settled',
    merchant: 'Northwind Market',
    fiat_amount: 1850,
    fiat_currency: 'USD',
    amount_stroops: 18_500_000,
    status: 'cleared',
    settledAt: new Date(Date.now() - 5 * 60 * 60 * 1000).toISOString(),
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
  const freezeApi = useCardFreezeAction({
    onSuccess: (action, txHash) => {
      console.info(`Card ${action} completed`, txHash);
    },
  });

  const isFrozen = freezeApi.isFrozen;
  const [rows, setRows] = useState<ActivityRow[]>(FIXTURE_ROWS);
  const [fxSnapshot, setFxSnapshot] = useState<CardFxSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [account, setAccount] = useState<CardAccountSummary | null>(FIXTURE_ACCOUNT);
  const [topUpOpen, setTopUpOpen] = useState(false);
  const [topUpAmount, setTopUpAmount] = useState('');
  const [topUpError, setTopUpError] = useState<string | null>(null);
  const canUnfreeze = Boolean(session.isConnected);

  useEffect(() => {
    setFxSnapshot(computeCardFxQuotes(Date.now()));
  }, []);

  const handleRefreshRates = useCallback(() => {
    setFxSnapshot((current) =>
      computeCardFxQuotes(Date.now(), (current?.revision ?? 0) + 1),
    );
  }, []);

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

  const dailyProgress = calculateDailySpendingProgress(rows, DAILY_LIMIT_USDC);

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

  return (
    <main className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-12">
      <section
        className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950"
        data-testid="card-balance-section"
      >
        <h2 className="text-lg font-semibold text-slate-900 dark:text-slate-50">
          Your card
        </h2>
        <div className="mb-4">
          <CardNicknameEditor stellarAddress={session.address} />
        </div>
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
          <CardFreezeButton api={freezeApi} />
        </div>

        <div className="mt-6 rounded-xl border border-slate-200 bg-slate-50 p-4 text-sm text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
          {canUnfreeze
            ? 'Wallet connected. Unfreeze can be used at any time.'
            : 'Connect a wallet to enable Unfreeze.'}
        </div>

        {/* ── Daily spending meter (CARD-49) ─────────────────────────── */}
        <CardDailyLimitProgress progress={dailyProgress} className="mt-6" />
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div className="mb-4 flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-slate-900 dark:text-slate-50">
            Indicative FX rates
          </h2>
          <span className="text-xs uppercase tracking-[0.2em] text-slate-500">
            Not a locked price
          </span>
        </div>

        {fxSnapshot ? (
          <div className="flex flex-col gap-4">
            <CardFxFreshnessBadge
              rate={formatCardFxRate(
                getCardFxQuote(fxSnapshot, FEATURED_FIAT_CURRENCY),
              )}
              retrievedAtMs={fxSnapshot.retrievedAtMs}
              onRefresh={handleRefreshRates}
            />

            <div className="overflow-hidden rounded-xl border border-slate-200 dark:border-slate-800">
              <div className="grid grid-cols-[1fr_1fr] gap-3 bg-slate-50 px-4 py-3 text-xs font-medium uppercase tracking-[0.12em] text-slate-500 dark:bg-slate-900 dark:text-slate-400">
                <span>Currency</span>
                <span>Rate per USDC</span>
              </div>
              {fxSnapshot.quotes.map((quote) => (
                <div
                  key={quote.fiatCurrency}
                  className="grid grid-cols-[1fr_1fr] gap-3 border-t border-slate-200 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:text-slate-200"
                >
                  <span className="font-medium text-slate-900 dark:text-slate-50">
                    {quote.fiatCurrency}
                  </span>
                  <span className="font-mono">{quote.ratePerUsdc}</span>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="rounded-xl border border-dashed border-slate-200 p-6 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
            Loading indicative rates…
          </div>
        )}
      </section>

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
