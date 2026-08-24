import type { AdapterNetworkId, ChainFamily } from '@/lib/wallet/adapters';

/** Display chain ids shown in corridor UI (may map 1:1 to ChainFamily). */
export type ChainDisplayId =
  | 'stellar'
  | 'ethereum-sepolia'
  | 'solana'
  | 'bitcoin'
  | 'tron';

export type CorridorId =
  | 'stellar-native'
  | 'evm-to-stellar'
  | 'stellar-to-evm'
  | 'solana-to-stellar'
  | 'bitcoin-to-stellar'
  | 'tron-to-stellar';

/** Explicit id when source/destination pair is not in the catalog. */
export type UnmatchedCorridorId = 'unmatched';

export type CorridorSelectionId = CorridorId | UnmatchedCorridorId;

export type CorridorAvailability = 'executable' | 'coming_soon';

export type ResolvedCorridorAvailability =
  | CorridorAvailability
  | 'unsupported';

export type CrossChainProtocol = 'stellar-native' | 'cctp-preview';

export interface ChainDefinition {
  id: ChainDisplayId;
  chainFamily: ChainFamily;
  label: string;
  shortLabel: string;
  networkId: AdapterNetworkId;
  assetLabel: string;
  /** Default asset id for AmountInput heuristics on this leg. */
  defaultAssetId: string;
}

export interface CorridorDefinition {
  id: CorridorId;
  label: string;
  description: string;
  sourceChainId: ChainDisplayId;
  destChainId: ChainDisplayId;
  protocol: CrossChainProtocol;
  /** Catalog intent — always cross-check with `hasBackendRoute`. */
  catalogAvailability: CorridorAvailability;
}

export type ExecutionTimelineStepId =
  | 'stellar_swap'
  | 'burn'
  | 'attest'
  | 'mint';

export type ExecutionTimelineStepStatus =
  | 'unavailable'
  | 'pending'
  | 'active'
  | 'complete'
  | 'failed';

export interface ExecutionTimelineStep {
  id: ExecutionTimelineStepId;
  label: string;
  description: string;
  status: ExecutionTimelineStepStatus;
  /** Explorer or support link — only when a real tx exists. */
  href?: string;
  supportReference?: string;
  retryable?: boolean;
}

export type RecipientValidationResult =
  | { valid: true }
  | { valid: false; message: string };
