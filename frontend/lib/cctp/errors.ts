import { StellarRouteApiError } from '@/lib/api/client';
import { isUserRejection, WalletAdapterError } from '@/lib/wallet/adapters';
import { HorizonSubmitError } from '@/lib/wallet/submit';

export type CctpErrorKind =
  | 'retryable'
  | 'nonretryable'
  | 'wallet_rejection'
  | 'wrong_network'
  | 'quote_expired'
  | 'payload_expired'
  | 'sequence_stale'
  | 'provider_killed'
  | 'dependency_unavailable'
  | 'authorization_lost'
  | 'pending_ambiguous';

export interface CctpTraderError {
  kind: CctpErrorKind;
  title: string;
  message: string;
  requestId?: string;
  action?: string;
}

export function mapCctpError(err: unknown): CctpTraderError {
  const message = err instanceof Error ? err.message : '';
  if (
    (message && isUserRejection(message)) ||
    isWalletRejection(err)
  ) {
    return {
      kind: 'wallet_rejection',
      title: 'Signature cancelled',
      message: 'You declined the wallet request. Nothing was submitted.',
      action: 'Try again when ready',
    };
  }

  if (err instanceof WalletAdapterError) {
    if (err.code === 'network_mismatch') {
      return {
        kind: 'wrong_network',
        title: 'Wrong network',
        message:
          'Switch your wallet to the required network, then try again.',
        action: 'Switch network in wallet',
      };
    }
  }

  if (isStaleSequenceError(err)) {
    return {
      kind: 'sequence_stale',
      title: 'Account sequence out of date',
      message:
        'Your Stellar account changed while this burn was being prepared. Re-prepare the burn using the same quote if it is still valid.',
      action: 'Re-prepare transaction',
    };
  }

  if (err instanceof StellarRouteApiError) {
    const requestId = extractRequestId(err.details);
    switch (err.code) {
      case 'cctp_not_enabled':
      case 'dependency_unavailable':
        return {
          kind: 'dependency_unavailable',
          title: 'Bridge temporarily unavailable',
          message:
            'CCTP is not ready on this deployment. Wait a moment and refresh readiness.',
          requestId,
          action: 'Retry',
        };
      case 'provider_killed':
        return {
          kind: 'provider_killed',
          title: 'Provider paused',
          message:
            'Circle CCTP is temporarily disabled. Signing is blocked until service recovers.',
          requestId,
          action: 'Check status',
        };
      case 'transfer_not_found':
        return {
          kind: 'authorization_lost',
          title: 'Transfer authorization lost',
          message:
            'This transfer cannot be resumed without its access token. Start a new quote.',
          requestId,
          action: 'New quote',
        };
      case 'quote_expired':
        return {
          kind: 'quote_expired',
          title: 'Quote expired',
          message: 'Request a fresh quote before signing.',
          requestId,
          action: 'Refresh quote',
        };
      case 'payload_expired':
        return {
          kind: 'payload_expired',
          title: 'Payload expired',
          message: 'Prepare a new wallet payload before signing again.',
          requestId,
          action: 'Prepare again',
        };
      case 'attestation_pending':
        return {
          kind: 'pending_ambiguous',
          title: 'Attestation in progress',
          message: 'Circle is still attesting your burn. This can take a few minutes.',
          requestId,
        };
      case 'idempotency_conflict':
        return {
          kind: 'nonretryable',
          title: 'Quote already in progress',
          message:
            'This idempotency key was used with different transfer inputs. Start a new quote or wait for the prior attempt to finish.',
          requestId,
          action: 'New quote',
        };
      case 'reattest_cooldown':
      case 'reattest_conflict':
        return {
          kind: 'retryable',
          title: 'Re-attestation cooling down',
          message:
            'A re-attestation was requested recently. Wait for the cooldown before retrying.',
          requestId,
          action: 'Wait and retry',
        };
      default:
        if (
          err.message?.includes('USDC approval, not a burn') ||
          err.message?.includes('Prepare the burn')
        ) {
          return {
            kind: 'nonretryable',
            title: 'Approval recorded — burn is next',
            message: err.message,
            requestId,
            action: 'Prepare burn',
          };
        }
        if (
          err.message?.includes('quote has expired') ||
          err.message?.includes('request a new quote')
        ) {
          return {
            kind: 'quote_expired',
            title: 'Quote expired',
            message: 'Request a fresh quote before preparing or signing.',
            requestId,
            action: 'Refresh quote',
          };
        }
        if (
          err.message?.includes('Could not prepare source transaction') ||
          err.message?.includes('Insufficient USDC balance') ||
          err.message?.includes('active prepare already exists')
        ) {
          return {
            kind: 'nonretryable',
            title: 'Could not prepare transaction',
            message: err.message,
            requestId,
            action: 'Check balance and retry',
          };
        }
        if (
          err.message?.includes('not yet available for verification') ||
          err.message?.includes('On-chain verification failed')
        ) {
          return {
            kind: 'retryable',
            title: 'Confirming on-chain transaction',
            message:
              'Your transaction was submitted but the API has not confirmed it yet. Wait a few seconds and try again.',
            requestId,
            action: 'Retry',
          };
        }
        if (err.status === 409) {
          return {
            kind: 'nonretryable',
            title: 'Request conflict',
            message:
              err.message ||
              'This transfer step conflicts with an in-flight operation. Reconcile status before retrying.',
            requestId,
            action: 'Check status',
          };
        }
        if (err.status === 503) {
          return {
            kind: 'dependency_unavailable',
            title: 'Service unavailable',
            message: 'The bridge API is temporarily unavailable. Try again shortly.',
            requestId,
            action: 'Retry',
          };
        }
        if (err.status >= 500) {
          return {
            kind: 'retryable',
            title: 'Temporary error',
            message: 'Something went wrong on our side. You can retry safely.',
            requestId,
            action: 'Retry',
          };
        }
        return {
          kind: 'nonretryable',
          title: 'Transfer blocked',
          message: err.message || 'This transfer cannot continue.',
          requestId,
        };
    }
  }

  if (err instanceof Error) {
    if (/expired/i.test(err.message)) {
      return {
        kind: 'payload_expired',
        title: 'Payload expired',
        message: err.message,
        action: 'Prepare again',
      };
    }
    if (/timeout|ambiguous|pending/i.test(err.message)) {
      return {
        kind: 'pending_ambiguous',
        title: 'Submission uncertain',
        message: err.message,
        action: 'Check explorer',
      };
    }
    return {
      kind: 'nonretryable',
      title: 'Something went wrong',
      message: err.message || 'Unknown error',
    };
  }

  return {
    kind: 'nonretryable',
    title: 'Something went wrong',
    message: 'Unknown error',
  };
}

function isWalletRejection(err: unknown): boolean {
  if (!(err instanceof Error)) return false;
  const code = (err as Error & { code?: string }).code;
  return code === 'user_rejected' || /reject|denied|cancel/i.test(err.message);
}

export function isStaleSequenceError(err: unknown): boolean {
  if (err instanceof HorizonSubmitError) {
    return (
      err.code === 'tx_bad_seq' ||
      err.transactionCode === 'tx_bad_seq' ||
      err.transactionCode === 'bad_sequence'
    );
  }
  if (err instanceof Error) {
    const code = (err as Error & { code?: string }).code;
    if (code === 'tx_bad_seq' || code === 'bad_sequence') return true;
    const status = (err as Error & { status?: string }).status;
    if (status === 'bad_sequence') return true;
    const message = err.message.toLowerCase();
    return message.includes('tx_bad_seq') || message.includes('bad_sequence');
  }
  return false;
}

function extractRequestId(details: unknown): string | undefined {
  if (!details || typeof details !== 'object') return undefined;
  const id = (details as { request_id?: unknown }).request_id;
  return typeof id === 'string' ? id : undefined;
}
