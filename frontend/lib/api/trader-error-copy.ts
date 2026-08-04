import { StellarRouteApiError } from '@/lib/api/client';
import { HorizonSubmitError } from '@/lib/wallet/submit';
import type { ApiErrorCode } from '@/types';
import { isLifecycleError } from '@/lib/swap/lifecycle-error';

export interface TraderErrorCopy {
  headline: string;
  explanation: string;
  recoveryAction: string;
  ctaLabel: string;
}

const DEFAULT_COPY: TraderErrorCopy = {
  headline: 'We could not refresh this quote',
  explanation: 'Something unexpected happened while preparing your trade details.',
  recoveryAction: 'Refresh the quote, then try again.',
  ctaLabel: 'Refresh quote',
};

const API_ERROR_COPY: Record<ApiErrorCode, TraderErrorCopy> = {
  validation_error: {
    headline: 'Check your trade details',
    explanation: 'One or more inputs are outside the allowed format or range.',
    recoveryAction: 'Update the amount or pair, then refresh the quote.',
    ctaLabel: 'Review trade inputs',
  },
  invalid_asset: {
    headline: 'This asset pair is not available right now',
    explanation: 'The selected asset format or issuer could not be matched.',
    recoveryAction: 'Choose a supported asset pair and try again.',
    ctaLabel: 'Select another pair',
  },
  no_route: {
    headline: 'No executable route found',
    explanation: 'Current liquidity cannot complete this trade at the requested size.',
    recoveryAction: 'Try a smaller amount or a different pair.',
    ctaLabel: 'Adjust trade size',
  },
  stale_market_data: {
    headline: 'Market data is still updating',
    explanation: 'Fresh pricing is not available yet for this route.',
    recoveryAction: 'Wait a moment and refresh to fetch a current quote.',
    ctaLabel: 'Refresh in a few seconds',
  },
  rate_limit_exceeded: {
    headline: 'Quote refresh is temporarily limited',
    explanation: 'Too many quote requests were sent in a short window.',
    recoveryAction: 'Wait briefly before refreshing again.',
    ctaLabel: 'Try again shortly',
  },
  overloaded: {
    headline: 'Quote service is handling high traffic',
    explanation: 'Routing services are taking longer than normal to respond.',
    recoveryAction: 'Retry in a moment to request a fresh quote.',
    ctaLabel: 'Retry quote',
  },
  bad_request: {
    headline: 'We could not process this request',
    explanation: 'The quote request did not match the expected API format.',
    recoveryAction: 'Refresh and try again with updated trade inputs.',
    ctaLabel: 'Refresh quote',
  },
  unauthorized: {
    headline: 'Session check required',
    explanation: 'Your current request needs a valid session context.',
    recoveryAction: 'Reconnect wallet or reload the page, then retry.',
    ctaLabel: 'Reconnect wallet',
  },
  not_found: {
    headline: 'Requested market data was not found',
    explanation: 'The selected pair or route data is currently unavailable.',
    recoveryAction: 'Pick another pair and request a new quote.',
    ctaLabel: 'Choose another pair',
  },
  internal_error: {
    headline: 'Quote service hit an internal issue',
    explanation: 'The request reached the server but could not be completed safely.',
    recoveryAction: 'Retry shortly while we stabilize the route response.',
    ctaLabel: 'Retry quote',
  },
  network_error: {
    headline: 'Network connection interrupted',
    explanation: 'The app could not reach routing services from this device.',
    recoveryAction: 'Check your connection and refresh once online.',
    ctaLabel: 'Reconnect and refresh',
  },
  invalid_amount: {
    headline: 'Check the trade amount',
    explanation: 'The requested amount is not valid for this pair.',
    recoveryAction: 'Enter a positive amount and refresh the quote.',
    ctaLabel: 'Update amount',
  },
  invalid_slippage: {
    headline: 'Check your slippage setting',
    explanation: 'The slippage tolerance is outside the allowed range.',
    recoveryAction: 'Adjust slippage in settings, then refresh.',
    ctaLabel: 'Adjust slippage',
  },
  invalid_asset_format: {
    headline: 'Asset format is not recognized',
    explanation: 'One of the selected assets uses an unsupported identifier shape.',
    recoveryAction: 'Choose a supported asset pair and try again.',
    ctaLabel: 'Select another pair',
  },
  not_executable: {
    headline: 'This route is not executable right now',
    explanation: 'Simulation or venue policy blocked this trade path.',
    recoveryAction: 'Refresh for another route or try a smaller amount.',
    ctaLabel: 'Refresh quote',
  },
  not_implemented: {
    headline: 'This action is not available yet',
    explanation: 'The requested operation is documented but not enabled on this API.',
    recoveryAction: 'Try again later or use a supported classic SDEX route.',
    ctaLabel: 'Choose another route',
  },
  quote_not_found: {
    headline: 'Prepared quote was not found',
    explanation: 'The prepare quote id is unknown or no longer valid.',
    recoveryAction: 'Refresh the quote and start the swap again.',
    ctaLabel: 'Refresh quote',
  },
  quote_expired: {
    headline: 'This quote expired',
    explanation: 'The prepared swap timed out before it could be submitted.',
    recoveryAction: 'Refresh for a new price, then confirm again.',
    ctaLabel: 'Refresh quote',
  },
  duplicate_quote: {
    headline: 'A swap is already in progress',
    explanation: 'This wallet already has an active prepare or submitted quote.',
    recoveryAction: 'Wait for the in-progress swap to settle, or check wallet activity.',
    ctaLabel: 'Check activity',
  },
  dependency_unavailable: {
    headline: 'Network dependency unavailable',
    explanation:
      'Horizon did not confirm the broadcast yet. Your signed swap may still be pending on-chain.',
    recoveryAction:
      'Tap Retry submit to send the same signed transaction again. Do not prepare a new swap.',
    ctaLabel: 'Retry submit',
  },
  unsupported_execution_mode: {
    headline: 'AMM and Soroban swaps are not supported yet',
    explanation: 'Live swaps currently support classic one-hop SDEX PathPayment only.',
    recoveryAction: 'Choose a direct SDEX quote and try again.',
    ctaLabel: 'Use classic SDEX route',
  },
  unsupported_route: {
    headline: 'This route shape is not supported',
    explanation: 'Multi-hop classic routes cannot be prepared in this build.',
    recoveryAction: 'Select a one-hop SDEX pair and refresh the quote.',
    ctaLabel: 'Choose one-hop route',
  },
  cctp_not_enabled: {
    headline: 'CCTP bridge unavailable',
    explanation: 'Cross-chain USDC bridging is not enabled on this API deployment.',
    recoveryAction: 'Retry when corridor readiness shows executable.',
    ctaLabel: 'Check status',
  },
  transfer_not_found: {
    headline: 'Transfer authorization lost',
    explanation: 'This transfer cannot be resumed without its access token.',
    recoveryAction: 'Start a new quote.',
    ctaLabel: 'New quote',
  },
  provider_killed: {
    headline: 'Bridge provider paused',
    explanation: 'Circle CCTP is temporarily disabled on this deployment.',
    recoveryAction: 'Wait for provider recovery before signing.',
    ctaLabel: 'Retry later',
  },
  payload_expired: {
    headline: 'Wallet payload expired',
    explanation: 'The prepared transaction is no longer valid.',
    recoveryAction: 'Prepare a fresh payload before signing.',
    ctaLabel: 'Prepare again',
  },
  attestation_pending: {
    headline: 'Attestation in progress',
    explanation: 'Circle is still attesting your burn transaction.',
    recoveryAction: 'Wait for attestation before minting.',
    ctaLabel: 'Keep waiting',
  },
  network_mismatch: {
    headline: 'Wallet network mismatch',
    explanation: 'Your wallet network does not match the prepared transaction.',
    recoveryAction: 'Switch networks in your wallet and try again.',
    ctaLabel: 'Switch network',
  },
  idempotency_conflict: {
    headline: 'Quote already in progress',
    explanation:
      'This idempotency key was reused with different transfer inputs.',
    recoveryAction: 'Start a new quote or wait for the prior attempt to finish.',
    ctaLabel: 'New quote',
  },
  reattest_cooldown: {
    headline: 'Re-attestation cooling down',
    explanation: 'A re-attestation was requested recently.',
    recoveryAction: 'Wait for the cooldown before retrying.',
    ctaLabel: 'Wait',
  },
  reattest_conflict: {
    headline: 'Re-attestation in progress',
    explanation: 'Another re-attestation claim is already active.',
    recoveryAction: 'Wait and check transfer status.',
    ctaLabel: 'Check status',
  },
  unknown_error: DEFAULT_COPY,
};

function inferHorizonError(errorMessage: string): TraderErrorCopy | null {
  const text = errorMessage.toLowerCase();

  if (text.includes('tx_bad_seq')) {
    return {
      headline: 'Account sequence is out of date',
      explanation: 'Your wallet account changed while this swap was being prepared.',
      recoveryAction: 'Refresh the quote and submit the swap again.',
      ctaLabel: 'Refresh and retry',
    };
  }

  if (text.includes('tx_bad_auth')) {
    return {
      headline: 'Transaction could not be authorized',
      explanation: 'The network rejected the transaction signature for this swap.',
      recoveryAction: 'Reconnect your wallet on the correct network and try again.',
      ctaLabel: 'Reconnect wallet',
    };
  }

  if (text.includes('op_no_trust')) {
    return {
      headline: 'Missing trustline for this asset',
      explanation: 'Your account cannot receive the destination asset yet.',
      recoveryAction: 'Add the required trustline in your wallet, then retry.',
      ctaLabel: 'Add trustline and retry',
    };
  }

  if (text.includes('op_underfunded')) {
    return {
      headline: 'Insufficient funds for this swap',
      explanation: 'Your account balance cannot cover the trade amount and network fees.',
      recoveryAction: 'Lower the amount or add funds, then try again.',
      ctaLabel: 'Adjust amount',
    };
  }

  if (text.includes('op_line_full')) {
    return {
      headline: 'Trustline limit reached for this asset',
      explanation: "Receiving this amount would exceed your account's trust limit for the destination asset.",
      recoveryAction: 'Increase your trustline limit or reduce the trade size, then try again.',
      ctaLabel: 'Adjust trustline or amount',
    };
  }

  if (text.includes('op_low_reserve')) {
    return {
      headline: 'Minimum account reserve required',
      explanation: 'Completing this trade would leave your account below the minimum XLM reserve.',
      recoveryAction: 'Add XLM to your account or reduce the trade size, then try again.',
      ctaLabel: 'Add funds and retry',
    };
  }

  if (text.includes('op_no_issuer')) {
    return {
      headline: 'Asset issuer could not be found',
      explanation: 'The issuing account for this asset is missing or no longer valid.',
      recoveryAction: 'Choose a different asset pair and try again.',
      ctaLabel: 'Select another pair',
    };
  }

  if (text.includes('op_no_destination')) {
    return {
      headline: 'Destination account does not exist',
      explanation: 'The receiving account for this trade has not been created on the network.',
      recoveryAction: 'Confirm the destination account, then try again.',
      ctaLabel: 'Check destination and retry',
    };
  }

  if (text.includes('tx_insufficient_balance')) {
    return {
      headline: 'Not enough balance to cover this trade',
      explanation: 'Your account balance cannot cover the trade amount plus the required minimum reserve.',
      recoveryAction: 'Lower the amount or add funds, then try again.',
      ctaLabel: 'Adjust amount',
    };
  }

  if (text.includes('tx_insufficient_fee')) {
    return {
      headline: 'Network fee was too low',
      explanation: "The transaction fee did not meet the network's current minimum requirement.",
      recoveryAction: 'Refresh the quote to get an updated fee, then resubmit.',
      ctaLabel: 'Refresh and resubmit',
    };
  }

  if (text.includes('tx_too_late')) {
    return {
      headline: 'This quote expired before it was submitted',
      explanation: "The transaction's submission window closed before the network could process it.",
      recoveryAction: 'Refresh the quote to get a new submission window, then try again.',
      ctaLabel: 'Refresh quote',
    };
  }

  if (text.includes('invoke_host_function_trapped')) {
    return {
      headline: 'The swap contract could not complete this trade',
      explanation: 'The contract stopped unexpectedly while executing this swap.',
      recoveryAction: 'Try a different amount or pair, then submit again.',
      ctaLabel: 'Adjust trade and retry',
    };
  }

  if (text.includes('invoke_host_function_resource_limit_exceeded')) {
    return {
      headline: 'This trade is too complex to execute right now',
      explanation: 'The swap needs more computing resources than the network currently allows in one transaction.',
      recoveryAction: 'Try a smaller trade or a simpler route, then try again.',
      ctaLabel: 'Simplify trade',
    };
  }

  if (text.includes('invoke_host_function_entry_archived')) {
    return {
      headline: 'Contract data needs to be restored first',
      explanation: 'Some contract data required for this swap is archived and was not restored.',
      recoveryAction: 'Refresh the quote so it can include the restore step, then try again.',
      ctaLabel: 'Refresh quote',
    };
  }

  if (text.includes('transaction timed out') || text.includes('timed out')) {
    return {
      headline: 'Transaction timed out',
      explanation: 'Horizon did not confirm your transaction within 60 seconds.',
      recoveryAction: 'You can resubmit the swap or dismiss and refresh the quote.',
      ctaLabel: 'Resubmit swap',
    };
  }

  return null;
}

function inferWalletError(errorMessage: string): TraderErrorCopy | null {
  const text = errorMessage.toLowerCase();

  if (
    text.includes('wallet') ||
    text.includes('freighter') ||
    text.includes('xbull') ||
    text.includes('albedo') ||
    text.includes('lobstr') ||
    text.includes('rejected') ||
    text.includes('denied') ||
    text.includes('signature')
  ) {
    return {
      headline: 'Wallet action was not completed',
      explanation: 'The wallet did not confirm the request needed to continue.',
      recoveryAction: 'Reopen your wallet, approve the request, and submit again.',
      ctaLabel: 'Open wallet and retry',
    };
  }

  return null;
}

function inferNetworkError(errorMessage: string): TraderErrorCopy | null {
  const text = errorMessage.toLowerCase();

  if (
    text.includes('network') ||
    text.includes('timeout') ||
    text.includes('failed to fetch') ||
    text.includes('offline')
  ) {
    return API_ERROR_COPY.network_error;
  }

  return null;
}

const ACTIVE_PREPARE_COPY: TraderErrorCopy = {
  headline: 'An active prepare already exists',
  explanation:
    'This wallet already has a prepared swap that has not expired yet.',
  recoveryAction:
    'Finish or wait for the active prepare to expire before preparing again.',
  ctaLabel: 'Check activity',
};

const CONFIRM_TIMEOUT_COPY: TraderErrorCopy = {
  headline: 'Confirmation timed out',
  explanation:
    'Horizon did not confirm the transaction in time, but submission may still reconcile on-chain.',
  recoveryAction:
    'Check wallet activity before preparing or submitting again.',
  ctaLabel: 'Check activity',
};

const BAD_SEQUENCE_COPY: TraderErrorCopy = {
  headline: 'Account sequence is out of date',
  explanation:
    'Your wallet account changed while this swap was being prepared.',
  recoveryAction: 'Refresh the quote and submit the swap again.',
  ctaLabel: 'Refresh and retry',
};

const MISSING_NETWORK_PASSPHRASE_COPY: TraderErrorCopy = {
  headline: 'Prepared swap is missing network details',
  explanation:
    'The prepare response did not include a network passphrase required for safe signing.',
  recoveryAction: 'Refresh the quote and try again.',
  ctaLabel: 'Refresh quote',
};

const SUBMITTING_WITHOUT_HASH_COPY: TraderErrorCopy = {
  headline: 'Previous submit is still in progress',
  explanation:
    'A matching quote is locked in submitting state without a confirmed transaction hash yet.',
  recoveryAction:
    'Wait and reconcile wallet activity before preparing or submitting again.',
  ctaLabel: 'Check activity',
};

const NETWORK_MISMATCH_COPY: TraderErrorCopy = {
  headline: 'Wallet network does not match',
  explanation:
    'Your wallet is on a different Stellar network than the prepared swap.',
  recoveryAction:
    'Switch your wallet to the correct network, refresh the quote, then try again.',
  ctaLabel: 'Switch network',
};

function copyForConflictStatus(
  status: string | undefined,
): TraderErrorCopy | null {
  if (status === 'active_prepare_exists') return ACTIVE_PREPARE_COPY;
  if (status === 'confirm_timeout') return CONFIRM_TIMEOUT_COPY;
  if (status === 'bad_sequence') return BAD_SEQUENCE_COPY;
  if (status === 'missing_network_passphrase') {
    return MISSING_NETWORK_PASSPHRASE_COPY;
  }
  if (status === 'submitting_without_hash') {
    return SUBMITTING_WITHOUT_HASH_COPY;
  }
  if (status === 'network_mismatch') return NETWORK_MISMATCH_COPY;
  if (status === 'already_submitted' || status === 'in_progress') {
    return {
      headline: 'This swap was already submitted',
      explanation: 'A matching quote is already in flight or settled.',
      recoveryAction: 'Check wallet activity before trying again.',
      ctaLabel: 'Check activity',
    };
  }
  if (status === 'pending_reconcile') {
    return API_ERROR_COPY.dependency_unavailable;
  }
  return null;
}

function copyFromApiCode(code: string | undefined): TraderErrorCopy | null {
  if (!code || code === 'unknown_error') return null;
  if (code === 'confirm_timeout') return CONFIRM_TIMEOUT_COPY;
  if (code in API_ERROR_COPY) {
    return API_ERROR_COPY[code as ApiErrorCode];
  }
  return null;
}

function lifecycleStatusFromApiError(
  error: StellarRouteApiError,
): string | undefined {
  const details = error.details;
  if (!details || typeof details !== 'object') return undefined;
  const status = (details as { status?: unknown }).status;
  return typeof status === 'string' ? status : undefined;
}

export function getTraderErrorCopy(error: unknown): TraderErrorCopy {
  if (error instanceof HorizonSubmitError) {
    const hints = [
      ...(error.operationCodes ?? []),
      error.transactionCode,
      error.code,
      error.message,
    ];
    for (const hint of hints) {
      if (!hint) continue;
      const horizonCopy = inferHorizonError(hint);
      if (horizonCopy) {
        return horizonCopy;
      }
    }
    if (error.code === 'timeout') {
      const timeoutCopy = inferHorizonError('transaction timed out');
      if (timeoutCopy) {
        return timeoutCopy;
      }
    }
    return API_ERROR_COPY.network_error;
  }

  if (error instanceof StellarRouteApiError) {
    const conflictCopy = copyForConflictStatus(
      lifecycleStatusFromApiError(error),
    );
    if (conflictCopy) return conflictCopy;

    if (
      error.code === 'not_executable' &&
      /op_no_trust/i.test(error.message)
    ) {
      return {
        headline: 'Missing asset trustline',
        explanation:
          'Your wallet has not trusted the destination asset yet, so Horizon rejected the swap.',
        recoveryAction:
          'In Freighter → Manage assets, add USDy (issuer …FYDDGS), then refresh and swap again.',
        ctaLabel: 'Add trustline',
      };
    }

    const byCode = copyFromApiCode(error.code);
    if (byCode) return byCode;

    if (error.status === 400) return API_ERROR_COPY.bad_request;
    if (error.status === 401) return API_ERROR_COPY.unauthorized;
    if (error.status === 404) return API_ERROR_COPY.not_found;
    if (error.status === 429) return API_ERROR_COPY.rate_limit_exceeded;
    if (error.status >= 500) return API_ERROR_COPY.internal_error;

    return API_ERROR_COPY[error.code] ?? DEFAULT_COPY;
  }

  if (isLifecycleError(error)) {
    const conflictCopy = copyForConflictStatus(error.status);
    if (conflictCopy) return conflictCopy;
    const byCode = copyFromApiCode(error.code);
    if (byCode) return byCode;

    const horizonCopy = inferHorizonError(error.message);
    if (horizonCopy) return horizonCopy;
    const walletCopy = inferWalletError(error.message);
    if (walletCopy) return walletCopy;
    const networkCopy = inferNetworkError(error.message);
    if (networkCopy) return networkCopy;
  }

  if (
    error &&
    typeof error === 'object' &&
    'status' in error &&
    typeof (error as { status?: unknown }).status === 'number'
  ) {
    const status = (error as { status: number }).status;
    if (status === 400) return API_ERROR_COPY.bad_request;
    if (status === 401) return API_ERROR_COPY.unauthorized;
    if (status === 404) return API_ERROR_COPY.not_found;
    if (status === 429) return API_ERROR_COPY.rate_limit_exceeded;
    if (status >= 500) return API_ERROR_COPY.internal_error;
  }

  if (error instanceof Error) {
    const attachedStatus = (error as Error & { status?: unknown }).status;
    if (typeof attachedStatus === 'string') {
      const conflictCopy = copyForConflictStatus(attachedStatus);
      if (conflictCopy) return conflictCopy;
    }
    const attachedCode = (error as Error & { code?: unknown }).code;
    if (typeof attachedCode === 'string') {
      const byCode = copyFromApiCode(attachedCode);
      if (byCode) return byCode;
    }

    const horizonCopy = inferHorizonError(error.message);
    if (horizonCopy) {
      return horizonCopy;
    }

    const walletCopy = inferWalletError(error.message);
    if (walletCopy) {
      return walletCopy;
    }

    const networkCopy = inferNetworkError(error.message);
    if (networkCopy) {
      return networkCopy;
    }
  }

  return DEFAULT_COPY;
}

export function toTraderErrorLine(copy: TraderErrorCopy): string {
  return `${copy.headline}. ${copy.explanation} ${copy.recoveryAction}`;
}
