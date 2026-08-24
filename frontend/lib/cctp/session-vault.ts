const VAULT_KEY = 'stellarroute:cctp:v1';
const VAULT_VERSION = 2;
const DEFAULT_TTL_MS = 2 * 60 * 60 * 1000;
const PENDING_EVM_TX_TTL_MS = 30 * 60 * 1000;

import type { CctpWalletRoleBindings } from './wallet-role-binding';

export type BurnPrepareStep =
  | 'unknown'
  | 'approval_ready'
  | 'burn_ready'
  | 'reprepare_required';

export interface CctpPendingEvmTx {
  txHash: string;
  purpose: 'approval' | 'burn' | 'mint';
  expiresAt: number;
}

export interface CctpSessionRecoveryMeta {
  corridorId: string;
  direction: 'stellar_to_evm' | 'evm_to_stellar';
  sourceChainId: string;
  destChainId: string;
  amount: string;
  recipient: string;
  quoteExpiresAt?: number;
  burnPrepareStep?: BurnPrepareStep;
  lastPreparedFingerprint?: string;
  pendingEvmTx?: CctpPendingEvmTx;
  /** Public signer bindings captured at quote time (v2+). */
  walletBindings?: CctpWalletRoleBindings;
}

export interface CctpSessionRecord {
  version: 1 | 2;
  transferId: string;
  accessToken: string;
  idempotencyKey: string;
  createdAt: number;
  expiresAt: number;
  recovery: CctpSessionRecoveryMeta;
}

export type CctpSessionLoadResult =
  | { ok: true; record: CctpSessionRecord }
  | { ok: false; reason: 'missing' | 'invalid' | 'expired' | 'terminal' };

const TERMINAL_PURGE_STATUSES = new Set([
  'completed',
  'cancelled',
  'provider_killed',
]);

function isBrowserSession(): boolean {
  return typeof window !== 'undefined' && typeof sessionStorage !== 'undefined';
}

function validateRecord(raw: unknown): CctpSessionRecord | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Partial<CctpSessionRecord>;
  if (r.version !== 1 && r.version !== 2) return null;
  if (!r.transferId || typeof r.transferId !== 'string') return null;
  if (!r.accessToken || typeof r.accessToken !== 'string') return null;
  if (!r.idempotencyKey || typeof r.idempotencyKey !== 'string') return null;
  if (typeof r.createdAt !== 'number' || typeof r.expiresAt !== 'number') {
    return null;
  }
  if (!r.recovery || typeof r.recovery !== 'object') return null;
  const rec = r.recovery as Partial<CctpSessionRecoveryMeta>;
  if (!rec.corridorId || !rec.direction || !rec.amount || !rec.recipient) {
    return null;
  }
  const record = r as CctpSessionRecord;
  if (record.version === 2 && !rec.walletBindings) {
    return null;
  }
  return record;
}

export function saveCctpSession(record: CctpSessionRecord): void {
  if (!isBrowserSession()) return;
  sessionStorage.setItem(VAULT_KEY, JSON.stringify(record));
}

export function loadCctpSession(now = Date.now()): CctpSessionLoadResult {
  if (!isBrowserSession()) {
    return { ok: false, reason: 'missing' };
  }
  const raw = sessionStorage.getItem(VAULT_KEY);
  if (!raw) return { ok: false, reason: 'missing' };
  try {
    const parsed = validateRecord(JSON.parse(raw));
    if (!parsed) {
      clearCctpSession();
      return { ok: false, reason: 'invalid' };
    }
    if (parsed.expiresAt <= now) {
      clearCctpSession();
      return { ok: false, reason: 'expired' };
    }
    if (
      parsed.recovery.pendingEvmTx &&
      parsed.recovery.pendingEvmTx.expiresAt <= now
    ) {
      parsed.recovery.pendingEvmTx = undefined;
      saveCctpSession(parsed);
    }
    return { ok: true, record: parsed };
  } catch {
    clearCctpSession();
    return { ok: false, reason: 'invalid' };
  }
}

export function patchCctpSessionRecovery(
  patch: Partial<CctpSessionRecoveryMeta>,
  now = Date.now(),
): CctpSessionRecord | null {
  const loaded = loadCctpSession(now);
  if (!loaded.ok) return null;
  const next: CctpSessionRecord = {
    ...loaded.record,
    recovery: { ...loaded.record.recovery, ...patch },
  };
  saveCctpSession(next);
  return next;
}

export function setPendingEvmTx(
  input: Omit<CctpPendingEvmTx, 'expiresAt'> & { ttlMs?: number },
  now = Date.now(),
): CctpSessionRecord | null {
  return patchCctpSessionRecovery({
    pendingEvmTx: {
      txHash: input.txHash,
      purpose: input.purpose,
      expiresAt: now + (input.ttlMs ?? PENDING_EVM_TX_TTL_MS),
    },
  });
}

export function clearPendingEvmTx(): CctpSessionRecord | null {
  return patchCctpSessionRecovery({ pendingEvmTx: undefined });
}

export function clearCctpSession(): void {
  if (!isBrowserSession()) return;
  sessionStorage.removeItem(VAULT_KEY);
}

export function purgeCctpSessionIfTerminal(status: string): void {
  if (TERMINAL_PURGE_STATUSES.has(status)) {
    clearCctpSession();
  }
}

export function buildCctpSessionRecord(input: {
  transferId: string;
  accessToken: string;
  idempotencyKey: string;
  recovery: CctpSessionRecoveryMeta;
  quoteExpiresAt?: number;
  ttlMs?: number;
  now?: number;
}): CctpSessionRecord {
  const now = input.now ?? Date.now();
  const ttl = input.ttlMs ?? DEFAULT_TTL_MS;
  const quoteExpiryMs = input.quoteExpiresAt
    ? input.quoteExpiresAt * 1000
    : now + ttl;
  return {
    version: VAULT_VERSION,
    transferId: input.transferId,
    accessToken: input.accessToken,
    idempotencyKey: input.idempotencyKey,
    createdAt: now,
    expiresAt: Math.min(now + ttl, quoteExpiryMs + 30 * 60 * 1000),
    recovery: {
      burnPrepareStep: 'unknown',
      ...input.recovery,
    },
  };
}

/** Safe snapshot for UI — never includes access token. */
export function cctpSessionPublicView(
  record: CctpSessionRecord,
): Omit<CctpSessionRecord, 'accessToken'> & { hasToken: true } {
  const { accessToken: _token, ...rest } = record;
  return { ...rest, hasToken: true };
}

export function redactSecretsForLogs(value: unknown): string {
  const json = JSON.stringify(value);
  return json
    .replace(/"access_token"\s*:\s*"[^"]+"/gi, '"access_token":"[redacted]"')
    .replace(/"accessToken"\s*:\s*"[^"]+"/gi, '"accessToken":"[redacted]"');
}

/**
 * Session vault stores transfer access tokens in sessionStorage only.
 * Any XSS on this origin can exfiltrate in-flight CCTP sessions — deploy a
 * strict Content-Security-Policy and avoid inline script on swap surfaces.
 */
export const CCTP_SESSION_VAULT_SECURITY_NOTE =
  'CCTP access tokens live in sessionStorage and are cleared on terminal status.';

export function buildSessionRecoveryRevision(record: CctpSessionRecord): string {
  const r = record.recovery;
  return [
    record.transferId,
    record.idempotencyKey,
    r.burnPrepareStep ?? 'unknown',
    r.lastPreparedFingerprint ?? '',
    r.pendingEvmTx?.txHash ?? '',
  ].join(':');
}

/** Stable key for one-shot automatic GET reconciliation (excludes prepare-step churn). */
export function buildAutoReconcileRevision(record: CctpSessionRecord): string {
  const r = record.recovery;
  return [record.transferId, record.idempotencyKey, r.pendingEvmTx?.txHash ?? ''].join(
    ':',
  );
}

export function sessionHasWalletBindings(record: CctpSessionRecord): boolean {
  return Boolean(record.recovery.walletBindings);
}

export function sessionRequiresBindingRecovery(record: CctpSessionRecord): boolean {
  return record.version < 2 || !record.recovery.walletBindings;
}

export function sessionRecoveryMatchesInputs(
  record: CctpSessionRecord,
  input: {
    sourceChainId: string;
    destChainId: string;
    amount: string;
    recipient: string;
  },
): boolean {
  const r = record.recovery;
  return (
    r.sourceChainId === input.sourceChainId &&
    r.destChainId === input.destChainId &&
    r.amount === input.amount &&
    r.recipient === input.recipient
  );
}
