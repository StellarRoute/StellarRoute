import { PathStep } from './index';

export type TransactionStatus =
  | 'pending'
  | 'submitted'
  | 'confirmed'
  | 'failed'
  | 'dropped';

export type TimelinePhase = 'signature' | 'submit' | 'inclusion' | 'finality';

export type TimelineEventState =
  | 'active'
  | 'success'
  | 'failed'
  | 'retrying';

export type TimelineEventSource = 'wallet' | 'api' | 'chain';

export interface TransactionCorrelationIds {
  walletRequestId: string;
  apiRequestId?: string;
  txHash?: string;
  replacementTxHash?: string;
}

export interface TransactionTimelineEvent {
  id: string;
  phase: TimelinePhase;
  state: TimelineEventState;
  source: TimelineEventSource;
  timestamp: number;
  titleKey: string;
  descriptionKey?: string;
  correlation: TransactionCorrelationIds;
  attempt: number;
  txHash?: string;
  replacedTxHash?: string;
  errorCode?: string;
  errorMessage?: string;
}

export interface TransactionRecord {
  id: string; // unique identifier (could be hash if known)
  timestamp: number; // unix timestamp
  
  // Trade Details
  fromAsset: string; // e.g., 'XLM'
  fromAmount: string; // e.g., '10.5'
  fromIcon?: string; // e.g., URL to icon or generic identifier
  
  toAsset: string; 
  toAmount: string;
  toIcon?: string;

  // Swap parameters
  exchangeRate: string;
  priceImpact: string;
  minReceived: string;
  networkFee: string;
  
  // Overall route info
  routePath: PathStep[];

  // Execution Status
  status: TransactionStatus;
  
  // Results / Errors
  hash?: string; // on-chain transaction hash
  errorMessage?: string; // reason for failure if applicable

  // Timeline / Correlation metadata
  timeline?: TransactionTimelineEvent[];
  correlation?: TransactionCorrelationIds;
  retryCount?: number;
  replacementCount?: number;
  
  walletAddress: string; // to track history per-wallet
}
