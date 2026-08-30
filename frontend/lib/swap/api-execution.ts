/**
 * Classic-only API swap execution: prepare → Freighter sign → submit → confirm.
 *
 * Scope: exactly one SDEX/Horizon hop (`execution_mode: classic_path_payment`).
 * Multi-hop and AMM/Soroban routes fail closed before prepare/sign.
 */

import type { PathStep, Asset } from '@/types';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';
import {
  StellarRouteApiError,
  stellarRouteClient,
  type PreparedSwapResponse,
  type StellarRouteClient,
  type SwapRouteHop,
  type SwapSubmitResponse,
} from '@/lib/api/client';
import {
  getHorizonUrl,
  getNetworkPassphrase,
  type HorizonSubmitResult,
} from '@/lib/wallet/submit';
import type { WalletNetwork } from '@/lib/wallet/types';
import { isProductionFrontendEnv } from '@/lib/env-guard';
import { isLifecycleError } from '@/lib/swap/lifecycle-error';

export const CLASSIC_EXECUTION_MODE = 'classic_path_payment' as const;

export const NETWORK_MISMATCH_COPY =
  'Wallet network does not match the prepared swap network. Switch your wallet to the correct network and refresh the quote before signing.';

export const MISSING_NETWORK_PASSPHRASE_COPY =
  'Prepared swap is missing a network passphrase. Refresh the quote and try again.';

export const REAL_XDR_DISABLED_PRODUCTION_COPY =
  'Live swaps require server prepare → wallet sign → API submit. Client-built XDR submission is disabled in production.';

export const REAL_XDR_DISABLED_COPY =
  'Live swaps require NEXT_PUBLIC_FLAG_REAL_XDR=true (API prepare → wallet sign → API submit). There is no client-built XDR fallback.';

export const REAL_XDR_LOADING_COPY =
  'Swap execution is still loading. Live swaps use API prepare → wallet sign → API submit only.';

export type SwapExecutionModeKind =
  | { mode: 'api_prepare_submit' }
  | { mode: 'disabled'; message: string };

/**
 * Resolve API prepare/submit vs fail-closed disable.
 * While flags are loading, fail closed (never an alternate client-XDR path).
 */
export function resolveSwapExecutionMode(options: {
  realXdrEnabled: boolean;
  flagsLoading?: boolean;
  isProduction?: boolean;
}): SwapExecutionModeKind {
  if (options.flagsLoading) {
    return { mode: 'disabled', message: REAL_XDR_LOADING_COPY };
  }
  if (options.realXdrEnabled) {
    return { mode: 'api_prepare_submit' };
  }
  const isProduction =
    options.isProduction ?? isProductionFrontendEnv(process.env);
  return {
    mode: 'disabled',
    message: isProduction
      ? REAL_XDR_DISABLED_PRODUCTION_COPY
      : REAL_XDR_DISABLED_COPY,
  };
}

export function assertPrepareNetworkPassphrase(
  preparedPassphrase: string | undefined | null,
  walletPassphrase: string | undefined | null,
): void {
  const prepared = preparedPassphrase?.trim() ?? '';
  if (!prepared) {
    const err = new Error(MISSING_NETWORK_PASSPHRASE_COPY) as Error & {
      code: string;
      status: string;
    };
    err.code = 'validation_error';
    err.status = 'missing_network_passphrase';
    throw err;
  }
  const wallet = walletPassphrase?.trim() ?? '';
  if (!wallet || wallet !== prepared) {
    const err = new Error(NETWORK_MISMATCH_COPY) as Error & {
      code: string;
      status: string;
    };
    err.code = 'validation_error';
    err.status = 'network_mismatch';
    throw err;
  }
}

export type ClassicPreflightReason =
  | 'ok'
  | 'missing_route'
  | 'multi_hop'
  | 'amm_or_soroban'
  | 'unsupported_source';

export interface ClassicPreflightResult {
  ok: boolean;
  reason: ClassicPreflightReason;
  message: string | null;
}

export function assetToCanonical(asset: Asset | string): string {
  if (typeof asset === 'string') return asset;
  if (asset.asset_type === 'native') return 'native';
  const code = asset.asset_code ?? '';
  const issuer = asset.asset_issuer ?? '';
  return issuer ? `${code}:${issuer}` : code;
}

export function pathStepsToRouteHops(path: PathStep[]): SwapRouteHop[] {
  return path.map((step) => ({
    from_asset: assetToCanonical(step.from_asset),
    to_asset: assetToCanonical(step.to_asset),
    source: step.source,
    fee_bps: step.fee_bps,
    price: step.price,
  }));
}

function isClassicSdexSource(source: string): boolean {
  const normalized = source.trim().toLowerCase();
  return (
    normalized === 'sdex' ||
    normalized === 'horizon' ||
    normalized.startsWith('sdex:') ||
    normalized.startsWith('horizon:')
  );
}

function isAmmOrSorobanSource(source: string): boolean {
  const normalized = source.trim().toLowerCase();
  return (
    normalized.startsWith('amm') ||
    normalized.startsWith('soroban') ||
    normalized.startsWith('router')
  );
}

/** Preflight: exactly one classic SDEX/Horizon hop. */
export function preflightClassicOneHop(path: PathStep[] | undefined | null): ClassicPreflightResult {
  if (!path || path.length === 0) {
    return {
      ok: false,
      reason: 'missing_route',
      message: 'No classic route is available for this trade yet. Refresh the quote.',
    };
  }
  if (path.length !== 1) {
    return {
      ok: false,
      reason: 'multi_hop',
      message:
        'Multi-hop routes are not supported yet. Choose a direct SDEX pair or wait for a one-hop quote.',
    };
  }
  const source = path[0]?.source ?? '';
  if (isAmmOrSorobanSource(source)) {
    return {
      ok: false,
      reason: 'amm_or_soroban',
      message:
        'AMM and Soroban routes are not supported for live swaps yet. Use a classic SDEX one-hop quote.',
    };
  }
  if (!isClassicSdexSource(source)) {
    return {
      ok: false,
      reason: 'unsupported_source',
      message: 'This venue is not supported for live swaps yet. Refresh for a classic SDEX quote.',
    };
  }
  return { ok: true, reason: 'ok', message: null };
}

export function conflictStatusFromDetails(details: unknown): string | undefined {
  if (!details || typeof details !== 'object') return undefined;
  const status = (details as { status?: unknown }).status;
  return typeof status === 'string' ? status : undefined;
}

export function isUserRejectionError(err: unknown): boolean {
  if (!(err instanceof Error)) return false;
  const msg = err.message.toLowerCase();
  return (
    msg.includes('reject') ||
    msg.includes('denied') ||
    msg.includes('user declined') ||
    msg.includes('cancelled') ||
    msg.includes('canceled')
  );
}

function swapErrorCode(err: unknown): string | undefined {
  if (err instanceof StellarRouteApiError) return err.code;
  if (isLifecycleError(err)) return err.code;
  if (err instanceof Error) {
    const code = (err as Error & { code?: unknown }).code;
    return typeof code === 'string' ? code : undefined;
  }
  return undefined;
}

function swapErrorStatus(err: unknown): string | undefined {
  if (isLifecycleError(err)) return err.status;
  if (err instanceof StellarRouteApiError) {
    return conflictStatusFromDetails(err.details);
  }
  if (err instanceof Error) {
    const status = (err as Error & { status?: unknown }).status;
    if (typeof status === 'string') return status;
    return conflictStatusFromDetails(
      (err as Error & { details?: unknown }).details,
    );
  }
  return undefined;
}

export function userCopyForSwapExecutionError(err: unknown): string {
  if (isUserRejectionError(err)) {
    return 'Signature cancelled — no transaction was submitted.';
  }

  const code = swapErrorCode(err);
  const conflictStatus = swapErrorStatus(err);

  if (code === 'unsupported_route' || code === 'unsupported_execution_mode') {
    return 'This route cannot be executed as a classic one-hop SDEX swap. Choose a direct SDEX quote.';
  }
  if (code === 'quote_expired' || code === 'stale_market_data') {
    return 'This quote expired. Refresh for a new price before swapping.';
  }
  if (
    conflictStatus === 'missing_network_passphrase' ||
    conflictStatus === 'network_mismatch'
  ) {
    return conflictStatus === 'missing_network_passphrase'
      ? MISSING_NETWORK_PASSPHRASE_COPY
      : NETWORK_MISMATCH_COPY;
  }
  if (conflictStatus === 'bad_sequence') {
    return 'Account sequence is out of date. Refresh the quote and submit the swap again.';
  }
  if (conflictStatus === 'submitting_without_hash') {
    return 'A previous submit is still in progress without a confirmed hash. Wait and reconcile before trying again.';
  }
  if (code === 'confirm_timeout' || conflictStatus === 'confirm_timeout') {
    return 'Confirmation timed out. Your submission may still reconcile on-chain — check wallet activity before preparing again.';
  }
  if (code === 'dependency_unavailable' || conflictStatus === 'pending_reconcile') {
    return 'Horizon did not confirm the broadcast yet. Use Retry submit (same signed transaction) — do not prepare a new swap.';
  }
  if (
    code === 'not_executable' &&
    err instanceof StellarRouteApiError &&
    /op_no_trust/i.test(err.message)
  ) {
    return 'Your wallet needs a USDy trustline before this swap can settle. Add USDy in Freighter (Manage assets), then refresh and try again.';
  }
  if (code === 'duplicate_quote' || conflictStatus === 'active_prepare_exists') {
    if (conflictStatus === 'active_prepare_exists') {
      return 'An active prepare already exists for this wallet. Finish or wait for it to expire before preparing again.';
    }
    if (conflictStatus === 'permanently_failed') {
      return 'This quote can no longer be submitted. Add any missing trustlines, refresh for a new quote, then try again.';
    }
    if (conflictStatus === 'already_submitted' || conflictStatus === 'in_progress') {
      return 'This swap was already submitted. Check wallet activity before trying again.';
    }
    if (code === 'duplicate_quote') {
      return 'This quote conflicts with an in-progress swap. Refresh and try again.';
    }
  }

  if (err instanceof StellarRouteApiError && err.message) return err.message;
  if (isLifecycleError(err) && err.message) return err.message;
  if (err instanceof Error && err.message) return err.message;
  return 'Swap failed. Refresh the quote and try again.';
}

export interface ApiSwapExecutionOptions {
  client?: StellarRouteClient;
  sender: string;
  slippageBps: number;
  network: WalletNetwork | null;
  signTransaction: (xdr: string) => Promise<string>;
  /** Max ambiguous submit retries after the first attempt (default: 4). */
  ambiguousSubmitRetries?: number;
  confirmOnHorizon?: boolean;
  confirmTimeoutMs?: number;
  confirmPollIntervalMs?: number;
}

export interface ApiSwapExecutionDeps {
  buildXdr: (params: TradeParams) => Promise<string>;
  signTransaction: (xdr: string) => Promise<string>;
  submitTransaction: (signedXdr: string) => Promise<HorizonSubmitResult>;
  getLastPrepare: () => PreparedSwapResponse | null;
  getLastSubmit: () => SwapSubmitResponse | null;
  /** Server amounts from the active prepare (for confirmation UI). */
  getPreparedAmounts: () => {
    expected_output: string;
    min_output?: string;
    quote_id: string;
    execution_mode: string;
  } | null;
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

async function confirmOnHorizon(
  txHash: string,
  network: WalletNetwork | null,
  timeoutMs: number,
  pollIntervalMs: number,
): Promise<void> {
  const horizonUrl = getHorizonUrl(network).replace(/\/$/, '');
  const url = `${horizonUrl}/transactions/${encodeURIComponent(txHash)}`;
  const deadline = Date.now() + timeoutMs;

  while (Date.now() <= deadline) {
    const response = await fetch(url, {
      method: 'GET',
      headers: { Accept: 'application/json' },
    });

    if (response.status === 404) {
      await sleep(pollIntervalMs);
      continue;
    }

    if (!response.ok) {
      throw new Error(`Horizon confirmation failed with HTTP ${response.status}`);
    }

    const body = (await response.json()) as { successful?: boolean; hash?: string };
    if (body.hash && body.hash !== txHash) {
      throw new Error(`Horizon hash mismatch: expected ${txHash}, got ${body.hash}`);
    }
    if (!body.successful) {
      throw new Error(`Transaction ${txHash} was not successful on Horizon`);
    }
    return;
  }

  const timeoutErr = new Error(
    'Confirmation timed out. Your submission may still reconcile on-chain — check wallet activity before preparing again.',
  ) as Error & { code: string; status: string };
  timeoutErr.code = 'confirm_timeout';
  timeoutErr.status = 'confirm_timeout';
  throw timeoutErr;
}

function isAmbiguousSubmitError(err: unknown): boolean {
  if (err instanceof StellarRouteApiError) {
    return (
      err.code === 'dependency_unavailable' ||
      err.code === 'network_error' ||
      err.status === 503 ||
      err.status === 0
    );
  }
  if (err instanceof Error) {
    const msg = err.message.toLowerCase();
    return msg.includes('timeout') || msg.includes('network');
  }
  return false;
}

/**
 * Build injectable lifecycle deps for classic API execution.
 * Always signs server `prepared.xdr_envelope` — no client-XDR fallback.
 */
export function createApiSwapExecution(
  options: ApiSwapExecutionOptions,
): ApiSwapExecutionDeps {
  const client = options.client ?? stellarRouteClient;
  let lastPrepare: PreparedSwapResponse | null = null;
  let lastSubmit: SwapSubmitResponse | null = null;
  let pendingQuoteId: string | null = null;
  let pendingSignedXdr: string | null = null;

  return {
    getLastPrepare: () => lastPrepare,
    getLastSubmit: () => lastSubmit,
    getPreparedAmounts: () =>
      lastPrepare
        ? {
            expected_output: lastPrepare.expected_output,
            min_output: lastPrepare.min_output,
            quote_id: lastPrepare.quote_id,
            execution_mode: lastPrepare.execution_mode,
          }
        : null,
    buildXdr: async (params: TradeParams) => {
      const preflight = preflightClassicOneHop(params.routePath);
      if (!preflight.ok) {
        throw new Error(preflight.message ?? 'Unsupported route for classic swap');
      }

      const hops = pathStepsToRouteHops(params.routePath);
      const prepared = await client.prepareSwap({
        route: { hops },
        amount: params.fromAmount,
        sender: params.walletAddress || options.sender,
        slippage_bps: options.slippageBps,
      });

      if (prepared.execution_mode !== CLASSIC_EXECUTION_MODE) {
        throw new StellarRouteApiError(
          422,
          'unsupported_execution_mode',
          `Unsupported execution_mode '${prepared.execution_mode}'`,
          { execution_mode: prepared.execution_mode },
        );
      }

      if (!prepared.xdr_envelope?.trim()) {
        throw new Error('Prepare returned an empty transaction envelope');
      }

      const walletPassphrase = options.network
        ? getNetworkPassphrase(options.network)
        : null;
      // Fail before Freighter signing when prepare network ≠ wallet network.
      assertPrepareNetworkPassphrase(
        prepared.network_passphrase,
        walletPassphrase,
      );

      lastPrepare = prepared;
      pendingQuoteId = prepared.quote_id;
      pendingSignedXdr = null;
      return prepared.xdr_envelope;
    },
    signTransaction: async (xdr: string) => {
      if (lastPrepare) {
        const walletPassphrase = options.network
          ? getNetworkPassphrase(options.network)
          : null;
        assertPrepareNetworkPassphrase(
          lastPrepare.network_passphrase,
          walletPassphrase,
        );
      }
      try {
        const signed = await options.signTransaction(xdr);
        if (!signed?.trim()) {
          throw new Error('Wallet returned an empty signed envelope');
        }
        pendingSignedXdr = signed;
        return signed;
      } catch (err) {
        if (isUserRejectionError(err)) {
          // Fail closed: do not submit after rejection.
          pendingSignedXdr = null;
        }
        throw err;
      }
    },
    submitTransaction: async (signedXdr: string) => {
      if (!pendingQuoteId) {
        throw new Error('Cannot submit swap without a prepared quote_id');
      }
      const envelope = pendingSignedXdr ?? signedXdr;
      if (!envelope?.trim()) {
        throw new Error('Cannot submit swap without a signed envelope');
      }

      const submitBody = {
        quote_id: pendingQuoteId,
        signed_xdr: envelope,
      };

      const maxAmbiguous = options.ambiguousSubmitRetries ?? 4;
      let submitted: SwapSubmitResponse | undefined;
      let lastErr: unknown;

      for (let attempt = 0; attempt <= maxAmbiguous; attempt++) {
        try {
          submitted = await client.submitSwap(submitBody);
          lastErr = undefined;
          break;
        } catch (err) {
          lastErr = err;
          if (!isAmbiguousSubmitError(err) || attempt >= maxAmbiguous) {
            break;
          }
          // Reuse exact same quote_id + signed_xdr — never re-prepare/re-sign.
          await sleep(500 * 2 ** attempt);
        }
      }

      if (!submitted) {
        // Only ambiguous Horizon outcomes become pending_reconcile + Resubmit.
        // Permanent failures (422 op_no_trust, 409 permanently_failed, etc.)
        // must keep their original status so the UI does not retry forever.
        if (
          lastErr instanceof StellarRouteApiError &&
          isAmbiguousSubmitError(lastErr)
        ) {
          throw new StellarRouteApiError(
            lastErr.status,
            lastErr.code,
            lastErr.message,
            {
              ...(typeof lastErr.details === 'object' && lastErr.details
                ? (lastErr.details as object)
                : {}),
              status: 'pending_reconcile',
              quote_id: pendingQuoteId,
            },
          );
        }
        pendingSignedXdr = null;
        pendingQuoteId = null;
        throw lastErr instanceof Error
          ? lastErr
          : new Error('Swap submit failed. Refresh the quote and try again.');
      }

      lastSubmit = submitted;
      // Keep quote_id bound for diagnostics; clear signed buffer after accept.
      pendingSignedXdr = envelope;

      if (options.confirmOnHorizon !== false) {
        await confirmOnHorizon(
          submitted.tx_hash,
          options.network,
          options.confirmTimeoutMs ?? 60_000,
          options.confirmPollIntervalMs ?? 2_000,
        );
      }

      return {
        hash: submitted.tx_hash,
        ledger: submitted.ledger,
      };
    },
  };
}
