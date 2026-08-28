import type {
  ChainDisplayId,
  ExecutionTimelineStep,
} from '@/lib/cross-chain/types';

export type CrossChainWalletStoryState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'mismatch'
  | 'unsupported';

export interface CrossChainDeckStoryPresentation {
  initialSourceChainId?: ChainDisplayId;
  initialDestChainId?: ChainDisplayId;
  sourceWalletState?: CrossChainWalletStoryState;
  destWalletState?: CrossChainWalletStoryState;
  timelineSteps?: ExecutionTimelineStep[];
}

export const EXECUTING_TIMELINE_STORY_FIXTURE: ExecutionTimelineStep[] = [
  {
    id: 'stellar_swap',
    label: 'Stellar swap',
    description: 'Fixture: prior step completed in story preview.',
    status: 'complete',
    href: 'https://stellar.expert/explorer/testnet/tx/fixture-stellar',
    supportReference: 'SR-FIXTURE-001',
  },
  {
    id: 'burn',
    label: 'Burn',
    description: 'Fixture: burn submitted — awaiting attestation.',
    status: 'active',
    href: 'https://sepolia.etherscan.io/tx/fixture-burn',
    supportReference: 'SR-FIXTURE-002',
    retryable: true,
  },
  {
    id: 'attest',
    label: 'Attest',
    description: 'Fixture: attestation pending.',
    status: 'pending',
  },
  {
    id: 'mint',
    label: 'Mint',
    description: 'Fixture: mint not started.',
    status: 'unavailable',
  },
];
