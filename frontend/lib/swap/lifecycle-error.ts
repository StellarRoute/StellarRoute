/**
 * Small serializable error shape for the transaction lifecycle.
 * Preserves curated API `code` + allowlisted conflict `status` without
 * storing arbitrary backend `details` blobs.
 */

import { StellarRouteApiError } from '@/lib/api/client';

function statusFromDetails(details: unknown): unknown {
  if (!details || typeof details !== 'object') return undefined;
  return (details as { status?: unknown }).status;
}

/** Conflict / lifecycle statuses safe to surface for curated copy. */
export const SAFE_LIFECYCLE_STATUSES = [
  'active_prepare_exists',
  'already_submitted',
  'in_progress',
  'permanently_failed',
  'pending_reconcile',
  'confirm_timeout',
  'bad_sequence',
  'missing_network_passphrase',
  'submitting_without_hash',
  'network_mismatch',
] as const;

export type SafeLifecycleStatus = (typeof SAFE_LIFECYCLE_STATUSES)[number];

export interface LifecycleError {
  message: string;
  code?: string;
  status?: SafeLifecycleStatus;
}

const SAFE_STATUS_SET = new Set<string>(SAFE_LIFECYCLE_STATUSES);

export function sanitizeLifecycleStatus(
  status: unknown,
): SafeLifecycleStatus | undefined {
  if (typeof status !== 'string') return undefined;
  return SAFE_STATUS_SET.has(status)
    ? (status as SafeLifecycleStatus)
    : undefined;
}

/** Plain lifecycle object (not an Error instance). */
export function isLifecycleError(value: unknown): value is LifecycleError {
  if (!value || typeof value !== 'object' || value instanceof Error) {
    return false;
  }
  return typeof (value as { message?: unknown }).message === 'string';
}

function readAttachedCode(err: Error): string | undefined {
  const code = (err as Error & { code?: unknown }).code;
  return typeof code === 'string' && code.length > 0 ? code : undefined;
}

function readAttachedStatus(err: Error): SafeLifecycleStatus | undefined {
  const direct = sanitizeLifecycleStatus(
    (err as Error & { status?: unknown }).status,
  );
  if (direct) return direct;
  const details = (err as Error & { details?: unknown }).details;
  return sanitizeLifecycleStatus(statusFromDetails(details));
}

/**
 * Convert thrown values into a serializable lifecycle error.
 * Never copies arbitrary `details` — only allowlisted `status`.
 */
export function toLifecycleError(err: unknown): LifecycleError {
  if (isLifecycleError(err)) {
    return {
      message: err.message || 'Unknown error',
      ...(err.code ? { code: err.code } : {}),
      ...(sanitizeLifecycleStatus(err.status)
        ? { status: sanitizeLifecycleStatus(err.status) }
        : {}),
    };
  }

  if (err instanceof StellarRouteApiError) {
    const status = sanitizeLifecycleStatus(statusFromDetails(err.details));
    return {
      message: err.message || 'Request failed',
      code: err.code,
      ...(status ? { status } : {}),
    };
  }

  if (err instanceof Error) {
    const code = readAttachedCode(err);
    const status = readAttachedStatus(err);
    return {
      message: err.message || 'Unknown error',
      ...(code ? { code } : {}),
      ...(status ? { status } : {}),
    };
  }

  return { message: 'Unknown error' };
}
