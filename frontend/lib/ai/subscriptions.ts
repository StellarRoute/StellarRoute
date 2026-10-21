/**
 * Due subscription confirm (AI-27).
 *
 * Paying a due subscription is the send tool plus a ledger update: build the
 * same unsigned Payment as AI-16, hand it to the wallet signer (AI-17), and only
 * once a transaction hash exists advance the schedule. A rejected signature
 * throws and leaves the subscription unchanged. Nothing here runs on a timer.
 */

import { buildSendPaymentXdr, type MemoInput } from './tools/send';

export type SubscriptionInterval = 'daily' | 'weekly' | 'monthly' | 'yearly';

export interface SubscriptionPayment {
  hash: string;
  paidAt: number;
}

export interface Subscription {
  id: string;
  payee: string;
  /** "native" or "CODE:ISSUER" */
  asset: string;
  amount: string;
  interval: SubscriptionInterval;
  maxPayments: number;
  paidCount: number;
  /** Epoch ms of the next due payment; null once the schedule is complete. */
  nextDueAt: number | null;
  payments: SubscriptionPayment[];
  memo?: MemoInput;
}

/** Signs the unsigned XDR in the user's wallet, submits it, and resolves to the tx hash. */
export type SignAndSubmit = (unsignedXdr: string) => Promise<string>;

export interface ConfirmDuePaymentParams {
  subscription: Subscription;
  sourceAddress: string;
  networkPassphrase: string;
  horizonUrl: string;
  signAndSubmit: SignAndSubmit;
  now?: number;
  /** Test hook — skips the Horizon sequence lookup in the send tool. */
  sequenceOverride?: bigint;
}

export function advanceDueDate(from: number, interval: SubscriptionInterval): number {
  const date = new Date(from);
  switch (interval) {
    case 'daily':
      date.setUTCDate(date.getUTCDate() + 1);
      break;
    case 'weekly':
      date.setUTCDate(date.getUTCDate() + 7);
      break;
    case 'monthly':
      date.setUTCMonth(date.getUTCMonth() + 1);
      break;
    case 'yearly':
      date.setUTCFullYear(date.getUTCFullYear() + 1);
      break;
  }
  return date.getTime();
}

export function isDue(subscription: Subscription, now: number = Date.now()): boolean {
  return (
    subscription.nextDueAt !== null &&
    subscription.paidCount < subscription.maxPayments &&
    subscription.nextDueAt <= now
  );
}

/**
 * Confirms a due payment. Resolves to the updated subscription only after the
 * signer returns a hash; any signer error propagates and nothing advances.
 */
export async function confirmDuePayment(params: ConfirmDuePaymentParams): Promise<Subscription> {
  const { subscription } = params;
  const now = params.now ?? Date.now();

  if (!isDue(subscription, now)) {
    throw new Error('Subscription is not due');
  }

  const unsignedXdr = await buildSendPaymentXdr({
    sourceAddress: params.sourceAddress,
    destination: subscription.payee,
    asset: subscription.asset,
    amount: subscription.amount,
    memo: subscription.memo,
    networkPassphrase: params.networkPassphrase,
    horizonUrl: params.horizonUrl,
    sequenceOverride: params.sequenceOverride,
  });

  const hash = await params.signAndSubmit(unsignedXdr);
  if (!hash) {
    throw new Error('Wallet returned no transaction hash');
  }

  const paidCount = subscription.paidCount + 1;
  return {
    ...subscription,
    paidCount,
    payments: [...subscription.payments, { hash, paidAt: now }],
    nextDueAt:
      paidCount >= subscription.maxPayments
        ? null
        : advanceDueDate(subscription.nextDueAt as number, subscription.interval),
  };
}
