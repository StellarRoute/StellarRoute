import { PathStep } from './index';

export type TransactionStatus =
  | 'pending'
  | 'submitted'
  | 'confirmed'
  | 'failed'
  | 'dropped';

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
  
  walletAddress: string; // to track history per-wallet
}
