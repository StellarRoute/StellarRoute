import type { CctpTransferStatusResponse } from '@/lib/cctp/types';
import type { ExecutionTimelineStep } from './types';

/**
 * Map live CCTP transfer status onto the execution timeline so Burn → Attest → Mint
 * reflects durable backend progress (not the static preview rail).
 */
export function buildCctpTimelineFromTransfer(
  transfer: CctpTransferStatusResponse,
): ExecutionTimelineStep[] {
  const status = transfer.status;
  const burnHref = transfer.source_tx_hash
    ? sourceTxUrl(transfer.source_tx_hash, transfer.direction)
    : undefined;
  const mintHref = transfer.destination_tx_hash
    ? destTxUrl(transfer.destination_tx_hash, transfer.direction)
    : undefined;

  const burn = stepStatus(status, 'burn');
  const attest = stepStatus(status, 'attest');
  const mint = stepStatus(status, 'mint');

  return [
    {
      id: 'stellar_swap',
      label: 'Stellar swap',
      description: 'Not used for CCTP USDC bridge legs.',
      status: 'unavailable',
    },
    {
      id: 'burn',
      label: 'Burn',
      description:
        burn === 'complete'
          ? 'Source USDC burned / locked.'
          : burn === 'active'
            ? 'Sign and submit the source burn.'
            : 'Source-chain burn.',
      status: burn,
      href: burnHref,
      supportReference: transfer.support_reference_id,
    },
    {
      id: 'attest',
      label: 'Attest',
      description:
        attest === 'complete'
          ? 'Circle attestation received.'
          : attest === 'active'
            ? 'Waiting for Circle attestation relay.'
            : attest === 'failed'
              ? 'Attestation failed — retry when available.'
              : 'Circle attestation.',
      status: attest,
      supportReference: transfer.support_reference_id,
      retryable: status === 'attestation_failed' && transfer.retryable,
    },
    {
      id: 'mint',
      label: 'Mint',
      description:
        mint === 'complete'
          ? 'Destination USDC minted.'
          : mint === 'active'
            ? 'Sign mint on the destination chain.'
            : 'Destination mint after attestation.',
      status: mint,
      href: mintHref,
      supportReference: transfer.support_reference_id,
    },
  ];
}

function stepStatus(
  status: CctpTransferStatusResponse['status'],
  step: 'burn' | 'attest' | 'mint',
): ExecutionTimelineStep['status'] {
  if (status === 'completed') return 'complete';
  if (status === 'cancelled' || status === 'provider_killed') {
    return step === 'burn' ? 'failed' : 'unavailable';
  }

  if (step === 'burn') {
    if (
      status === 'created' ||
      status === 'burn_prepared' ||
      status === 'burn_submitted'
    ) {
      return status === 'burn_submitted' ? 'complete' : 'active';
    }
    return 'complete';
  }

  if (step === 'attest') {
    if (status === 'attestation_failed') return 'failed';
    if (status === 'awaiting_attestation') return 'active';
    if (
      status === 'attestation_ready' ||
      status === 'mint_prepared' ||
      status === 'mint_submitted' ||
      status === 'mint_failed_retryable'
    ) {
      return 'complete';
    }
    return 'pending';
  }

  // mint
  if (status === 'mint_failed_retryable') return 'failed';
  if (
    status === 'attestation_ready' ||
    status === 'mint_prepared' ||
    status === 'mint_submitted'
  ) {
    return 'active';
  }
  return 'pending';
}

function sourceTxUrl(
  hash: string,
  direction: CctpTransferStatusResponse['direction'],
) {
  const encoded = encodeURIComponent(hash);
  if (direction === 'evm_to_stellar') {
    return `https://sepolia.etherscan.io/tx/${encoded}`;
  }
  return `https://stellar.expert/explorer/testnet/tx/${encoded}`;
}

function destTxUrl(
  hash: string,
  direction: CctpTransferStatusResponse['direction'],
) {
  const encoded = encodeURIComponent(hash);
  if (direction === 'stellar_to_evm') {
    return `https://sepolia.etherscan.io/tx/${encoded}`;
  }
  return `https://stellar.expert/explorer/testnet/tx/${encoded}`;
}
