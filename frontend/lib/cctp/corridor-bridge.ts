import type { ChainDisplayId } from '@/lib/cross-chain/types';
import { CHAIN_DEFINITIONS } from '@/lib/cross-chain/corridors';
import type { CctpDirection, CctpQuoteRequest } from './types';
import {
  CCTP_CORRIDOR_DEFAULTS,
  SEPOLIA_USDC,
  STELLAR_TESTNET_USDC,
} from './constants';
import { findExecutableCorridor } from './readiness';

export function displayIdToCaip(id: ChainDisplayId): string {
  return CHAIN_DEFINITIONS[id].networkId;
}

export function resolveCctpDirection(
  sourceChainId: ChainDisplayId,
  destChainId: ChainDisplayId,
): CctpDirection | null {
  if (sourceChainId === 'stellar' && destChainId === 'ethereum-sepolia') {
    return 'stellar_to_evm';
  }
  if (sourceChainId === 'ethereum-sepolia' && destChainId === 'stellar') {
    return 'evm_to_stellar';
  }
  return null;
}

export function buildCctpQuoteRequest(input: {
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  amount: string;
  recipient: string;
  sender?: string;
  mintSubmitter?: string;
}): CctpQuoteRequest | null {
  const direction = resolveCctpDirection(input.sourceChainId, input.destChainId);
  if (!direction) return null;

  const sourceCaip = displayIdToCaip(input.sourceChainId);
  const destCaip = displayIdToCaip(input.destChainId);
  const corridor =
    findExecutableCorridor(sourceCaip, destCaip) ??
    ({
      corridor_id: CCTP_CORRIDOR_DEFAULTS.corridorId,
      provider: CCTP_CORRIDOR_DEFAULTS.provider,
      direction,
      source_chain_id: sourceCaip,
      destination_chain_id: destCaip,
      source_asset:
        direction === 'stellar_to_evm' ? STELLAR_TESTNET_USDC : SEPOLIA_USDC,
      destination_asset:
        direction === 'stellar_to_evm' ? SEPOLIA_USDC : STELLAR_TESTNET_USDC,
      executable: false,
    } as const);

  return {
    corridor_id: corridor.corridor_id,
    provider: corridor.provider,
    direction,
    source_chain_id: corridor.source_chain_id,
    destination_chain_id: corridor.destination_chain_id,
    source_asset: corridor.source_asset,
    destination_asset: corridor.destination_asset,
    amount: input.amount,
    recipient: input.recipient,
    sender: input.sender,
    mint_submitter:
      direction === 'evm_to_stellar' ? input.mintSubmitter : undefined,
    // Both directions: Fast (~seconds when Iris attests). Standard remains available via API.
    finality: 'fast',
  };
}
