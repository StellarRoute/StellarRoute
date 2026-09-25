/**
 * Due subscription confirm (AI-27).
 *
 * Paying a due subscription is the send tool plus a ledger update: build the
 * same unsigned Payment as AI-16, hand it to the wallet signer (AI-17), and only
 * once a transaction hash exists advance the schedule. A rejected signature
 * throws and leaves the subscription unchanged. Nothing here runs on a timer.
 */

import { buildSendPaymentXdr, type MemoInput } from './tools/send';

export type SubscriptionInterval = 'daily' | 'weekly' | 'biweekly' | 'monthly' | 'quarterly' | 'yearly' | string;

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

// ==========================================
// AI-25: Local Storage Ledger Implementation
// ==========================================

export type SubscriptionStatus = 'active' | 'cancelled' | 'completed';

export interface SubscriptionRecord {
  /** Unique subscription identifier */
  id: string;
  /** Destination payee public key or address */
  payee: string;
  /** Asset identifier (e.g. USDC, XLM) */
  asset: string;
  /** Amount to transfer per recurrence */
  amount: string;
  /** Cadence interval between payments */
  interval: SubscriptionInterval;
  /** Maximum number of payment occurrences (null/undefined for indefinite) */
  max: number | null;
  /** Total number of payment occurrences created/executed so far */
  createdCount: number;
  /** ISO 8601 date string for the next scheduled occurrence */
  nextDueDate: string;
  /** ISO 8601 timestamp when subscription was registered */
  createdAt: string;
  /** ISO 8601 timestamp of last record update */
  updatedAt: string;
  /** Optional reference memo */
  memo?: string;
  /** Lifecycle status of the local subscription */
  status: SubscriptionStatus;
}

export interface CreateSubscriptionInput {
  payee: string;
  asset: string;
  amount: string | number;
  interval: SubscriptionInterval;
  max?: number | null;
  startDate?: string | Date;
  memo?: string;
  id?: string;
}

/** Storage key namespaced to prevent collisions */
export const AI_SUBSCRIPTIONS_STORAGE_KEY = 'stellarroute_ai_subscriptions_v1';

/** Custom DOM event name for reactive subscription updates */
export const AI_SUBSCRIPTIONS_EVENT = 'stellarroute:ai:subscriptions-updated';

// In-memory fallback store for SSR or environments without localStorage
let memoryStorageFallback: Record<string, string> = {};

/**
 * Checks if window.localStorage is accessible and functional
 */
function isStorageAvailable(): boolean {
  if (typeof window === 'undefined' || !window.localStorage) {
    return false;
  }
  try {
    const testKey = '__sr_test_ls__';
    window.localStorage.setItem(testKey, testKey);
    window.localStorage.removeItem(testKey);
    return true;
  } catch {
    return false;
  }
}

/**
 * Safely reads raw string data from localStorage or in-memory fallback
 */
function getStorageItem(key: string): string | null {
  if (isStorageAvailable()) {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return memoryStorageFallback[key] ?? null;
    }
  }
  return memoryStorageFallback[key] ?? null;
}

/**
 * Safely writes raw string data to localStorage or in-memory fallback
 */
function setStorageItem(key: string, value: string): void {
  if (isStorageAvailable()) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      memoryStorageFallback[key] = value;
    }
  } else {
    memoryStorageFallback[key] = value;
  }

  // Dispatch custom browser event if available
  if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
    try {
      window.dispatchEvent(new CustomEvent(AI_SUBSCRIPTIONS_EVENT, { detail: { key } }));
    } catch {
      // Ignore event dispatch failure in non-browser or mock environments
    }
  }
}

/**
 * Internal helper for testing storage parsing and corruption recovery
 */
export function _setRawStorageForTesting(key: string, raw: string): void {
  if (isStorageAvailable()) {
    try {
      window.localStorage.setItem(key, raw);
    } catch {
      // Fallback
    }
  }
  memoryStorageFallback[key] = raw;
}

/**
 * Generates a unique subscription ID
 */
export function generateSubscriptionId(): string {
  const timestamp = Date.now().toString(36);
  const randomPart = Math.random().toString(36).substring(2, 8);
  return `sub_${timestamp}_${randomPart}`;
}

/**
 * Computes the next scheduled due date following an interval.
 * Handles month-end clamping (e.g., Jan 31 -> Feb 28/29) and leap years.
 */
export function calculateNextDueDate(
  fromDate: string | Date = new Date(),
  interval: SubscriptionInterval = 'monthly'
): string {
  const d = typeof fromDate === 'string' ? new Date(fromDate) : new Date(fromDate.getTime());
  
  if (isNaN(d.getTime())) {
    return new Date().toISOString();
  }

  const normalized = (interval || 'monthly').toLowerCase().trim();

  if (normalized === 'daily') {
    d.setUTCDate(d.getUTCDate() + 1);
  } else if (normalized === 'weekly') {
    d.setUTCDate(d.getUTCDate() + 7);
  } else if (normalized === 'biweekly') {
    d.setUTCDate(d.getUTCDate() + 14);
  } else if (normalized === 'monthly') {
    const originalDay = d.getUTCDate();
    d.setUTCMonth(d.getUTCMonth() + 1);
    // If the day rolled over (e.g. Jan 31 -> March 2 in non-leap Feb), adjust to last day of target month
    if (d.getUTCDate() < originalDay) {
      d.setUTCDate(0); // sets to last day of previous month
    }
  } else if (normalized === 'quarterly') {
    const originalDay = d.getUTCDate();
    d.setUTCMonth(d.getUTCMonth() + 3);
    if (d.getUTCDate() < originalDay) {
      d.setUTCDate(0);
    }
  } else if (normalized === 'yearly') {
    const originalDay = d.getUTCDate();
    d.setUTCFullYear(d.getUTCFullYear() + 1);
    if (d.getUTCDate() < originalDay) {
      d.setUTCDate(0);
    }
  } else {
    // Check for custom day counts (e.g., "10 days", "every 14 days", or just numbers)
    const match = normalized.match(/(\d+)/);
    if (match) {
      const days = parseInt(match[1], 10);
      d.setUTCDate(d.getUTCDate() + (isNaN(days) || days <= 0 ? 30 : days));
    } else {
      // Default to 30 days fallback
      d.setUTCDate(d.getUTCDate() + 30);
    }
  }

  return d.toISOString();
}

/**
 * Loads all subscriptions from local storage.
 * Returns an empty array if storage is empty or contains malformed data.
 */
export function listSubscriptions(): SubscriptionRecord[] {
  const raw = getStorageItem(AI_SUBSCRIPTIONS_STORAGE_KEY);
  if (!raw) {
    return [];
  }
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      return parsed.filter((item): item is SubscriptionRecord => {
        return (
          typeof item === 'object' &&
          item !== null &&
          typeof item.id === 'string' &&
          typeof item.payee === 'string' &&
          typeof item.amount === 'string' &&
          typeof item.nextDueDate === 'string'
        );
      });
    }
    return [];
  } catch {
    return [];
  }
}

/**
 * Loads only active subscriptions.
 */
export function getActiveSubscriptions(): SubscriptionRecord[] {
  return listSubscriptions().filter(sub => sub.status === 'active');
}

/**
 * Retrieves a single subscription by ID.
 */
export function getSubscription(id: string): SubscriptionRecord | null {
  if (!id) return null;
  const list = listSubscriptions();
  return list.find(s => s.id === id) ?? null;
}

/**
 * Creates and persists a new subscription intent in the local ledger.
 */
export function createSubscription(input: CreateSubscriptionInput): SubscriptionRecord {
  const nowIso = new Date().toISOString();
  
  // Calculate next due date
  const nextDueDate = input.startDate 
    ? (typeof input.startDate === 'string' ? input.startDate : input.startDate.toISOString())
    : calculateNextDueDate(new Date(), input.interval);

  const amountStr = typeof input.amount === 'number' ? input.amount.toString() : (input.amount || '0');
  const maxCount = typeof input.max === 'number' && input.max > 0 ? input.max : null;

  const newRecord: SubscriptionRecord = {
    id: input.id || generateSubscriptionId(),
    payee: (input.payee || '').trim(),
    asset: (input.asset || 'USDC').toUpperCase().trim(),
    amount: amountStr,
    interval: input.interval || 'monthly',
    max: maxCount,
    createdCount: 0,
    nextDueDate,
    createdAt: nowIso,
    updatedAt: nowIso,
    memo: input.memo ? input.memo.trim() : undefined,
    status: 'active'
  };

  const current = listSubscriptions();
  // If an existing subscription with the same ID exists, update it; otherwise append
  const existingIndex = current.findIndex(s => s.id === newRecord.id);
  if (existingIndex >= 0) {
    current[existingIndex] = newRecord;
  } else {
    current.push(newRecord);
  }

  setStorageItem(AI_SUBSCRIPTIONS_STORAGE_KEY, JSON.stringify(current));
  return newRecord;
}

/**
 * Cancels a subscription by removing it completely from the local record.
 * As per acceptance criteria: "Cancel deletes the local record only."
 * 
 * @param id Subscription identifier
 * @returns true if removed, false if not found
 */
export function cancelSubscription(id: string): boolean {
  if (!id) return false;
  const current = listSubscriptions();
  const filtered = current.filter(s => s.id !== id);
  
  if (filtered.length === current.length) {
    return false; // Nothing was removed
  }

  setStorageItem(AI_SUBSCRIPTIONS_STORAGE_KEY, JSON.stringify(filtered));
  return true;
}

/**
 * Alias for cancelSubscription
 */
export const deleteSubscription = cancelSubscription;

/**
 * Records that a payment was executed for a given subscription.
 * - Increments createdCount
 * - If max payments reached, marks subscription status as 'completed'
 * - Advances nextDueDate to the subsequent cycle
 */
export function recordPaymentExecuted(id: string): SubscriptionRecord | null {
  const current = listSubscriptions();
  const index = current.findIndex(s => s.id === id);
  if (index === -1) return null;

  const sub = current[index];
  const newCount = sub.createdCount + 1;
  const isCompleted = sub.max !== null && sub.max !== undefined && newCount >= sub.max;
  
  const updated: SubscriptionRecord = {
    ...sub,
    createdCount: newCount,
    status: isCompleted ? 'completed' : sub.status,
    nextDueDate: isCompleted ? sub.nextDueDate : calculateNextDueDate(sub.nextDueDate, sub.interval),
    updatedAt: new Date().toISOString()
  };

  current[index] = updated;
  setStorageItem(AI_SUBSCRIPTIONS_STORAGE_KEY, JSON.stringify(current));
  return updated;
}

/**
 * Checks if a subscription is currently due for confirmation/execution.
 */
export function isPaymentDue(sub: SubscriptionRecord, referenceDate: Date = new Date()): boolean {
  if (sub.status !== 'active') return false;
  const due = new Date(sub.nextDueDate);
  if (isNaN(due.getTime())) return false;
  return referenceDate.getTime() >= due.getTime();
}

/**
 * Utility to clear all local subscriptions (used for testing or client reset)
 */
export function clearSubscriptions(): void {
  if (isStorageAvailable()) {
    try {
      window.localStorage.removeItem(AI_SUBSCRIPTIONS_STORAGE_KEY);
    } catch {
      // Fallback
    }
  }
  memoryStorageFallback = {};
  
  if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
    try {
      window.dispatchEvent(new CustomEvent(AI_SUBSCRIPTIONS_EVENT, { detail: { action: 'clear' } }));
    } catch {
      // Ignore
    }
  }
}
