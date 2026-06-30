// frontend/src/types/trade.ts
export type TradeAction = 'BUY' | 'SELL' | 'SWAP' | 'SEND' | 'RECEIVE';

export interface TradeRecord {
  id: string;
  txHash: string;
  timestamp: Date;
  action: TradeAction;
  amount: string;
  asset: string;
}