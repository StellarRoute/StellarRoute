import type { CctpTransferStatus } from '@/lib/cctp/types';
import type { CctpSagaStage } from '@/hooks/useCctpSaga';

const STATUS_LABELS: Record<string, string> = {
  created: 'Started',
  burn_prepared: 'Ready to lock',
  burn_submitted: 'Lock submitted',
  // Circle Standard Transfer from Ethereum/Sepolia waits ~65 blocks (~15–19 min).
  // EVM→Stellar quotes use Fast by default (~seconds).
  awaiting_attestation: 'Waiting for confirmation',
  attestation_ready: 'Ready to receive',
  mint_prepared: 'Ready to receive',
  mint_submitted: 'Receive submitted',
  completed: 'Complete',
  attestation_failed: 'Confirmation failed',
  mint_failed_retryable: 'Receive failed — retry',
  cancelled: 'Cancelled',
  provider_killed: 'Bridge paused',
};

export function formatCctpTraderStatus(
  stage: CctpSagaStage,
  status?: CctpTransferStatus | string,
): string {
  if (stage === 'completed' || status === 'completed') return 'Complete';
  if (stage === 'pending_reconcile') return 'Transaction pending';
  if (status && STATUS_LABELS[status]) return STATUS_LABELS[status];
  if (stage === 'sign_approval') return 'Approve USDC spend';
  if (stage === 'sign_burn') return 'Confirm lock';
  if (stage === 'sign_trustline') return 'Open USDC trustline';
  if (stage === 'sign_mint') return 'Confirm receive';
  if (stage === 'quoted') return 'Quote ready';
  return stage.replace(/_/g, ' ');
}

export function formatCctpFinalityLabel(finality: string): string {
  if (finality === 'fast') return 'Faster';
  if (finality === 'standard') return 'Standard timing';
  return finality;
}
