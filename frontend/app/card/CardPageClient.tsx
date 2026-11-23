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
    const cancelled = false;

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
  }, []);

  const dailyProgress = calculateDailySpendingProgress(rows, DAILY_LIMIT_USDC);

  return (
    <main className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-12">
      <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div className="mb-4">
          <CardNicknameEditor stellarAddress={session.address} />
        </div>
        <CardFace
          last4="4567"
          expiry="12/27"
          usdcAvailable="1,234.56"
          isFrozen={isFrozen}
        />
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
    </main>
  );
}
