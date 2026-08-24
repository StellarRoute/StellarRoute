import type { CctpDirection } from '@/lib/cctp/types';

export interface CctpCtaBlockInput {
  direction: CctpDirection | null;
  sourceAmount: string;
  destRecipientAddress: string;
  bridgeReady: boolean;
  readinessLoading: boolean;
  sagaPrimaryDisabled: boolean;
}

export function resolveDestinationWalletSetupHint(
  direction: CctpDirection | null,
  destRecipientAddress: string,
): string | null {
  if (direction !== 'stellar_to_evm') return null;
  if (destRecipientAddress) return null;
  return 'Connect your ETH Sepolia wallet to receive USDC.';
}

export function resolveCctpCtaHint(input: CctpCtaBlockInput): string | null {
  if (input.readinessLoading) {
    return 'Checking bridge availability…';
  }
  if (!input.bridgeReady) {
    return 'Bridge is not available on this API right now.';
  }
  if (!input.destRecipientAddress) {
    if (input.direction === 'stellar_to_evm') {
      return 'Connect your ETH Sepolia wallet to continue.';
    }
    if (input.direction === 'evm_to_stellar') {
      return 'Connect your Stellar wallet to continue.';
    }
    return 'Connect a destination wallet to continue.';
  }
  if (!input.sourceAmount.trim()) {
    return 'Enter a USDC amount to get a quote.';
  }
  return null;
}

/** Mirrors CrossChainSwapDeck disable rules. */
export function isCctpPrimaryActionDisabled(input: CctpCtaBlockInput): boolean {
  if (input.readinessLoading) return true;
  if (input.sagaPrimaryDisabled) return true;
  if (!input.destRecipientAddress) return true;
  if (!input.sourceAmount.trim()) return true;
  return false;
}
