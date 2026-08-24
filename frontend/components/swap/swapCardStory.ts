import type { SwapButtonState } from './SwapButton';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';

export type SwapCardStoryFixture =
  | 'idle'
  | 'quoting'
  | 'stale'
  | 'confirming'
  | 'error';

export const SWAP_CARD_STORY_WALLET_ADDRESS =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

const baseTradeParams: TradeParams = {
  fromAsset: 'XLM',
  toAsset: 'USDC',
  fromAmount: '10.00',
  toAmount: '1.0500',
  exchangeRate: '0.1050',
  priceImpact: '0.12',
  minReceived: '1.0447 USDC',
  networkFee: '0.00001 XLM',
  routePath: [],
  walletAddress: SWAP_CARD_STORY_WALLET_ADDRESS,
};

export interface SwapCardStoryPresentation {
  walletConnected: boolean;
  seedFromAmount?: string;
  buttonState?: SwapButtonState;
  quoteLoading?: boolean;
  quoteStale?: boolean;
  quoteError?: { message: string } | null;
  quotePriceImpact?: number;
  toAmount?: string;
  formattedRate?: string;
  confirmModalOpen?: boolean;
  optimisticStatus?: 'review' | 'pending' | 'submitted' | 'confirmed' | 'failed' | 'dropped';
  tradeParams?: TradeParams;
}

export function getSwapCardStoryPresentation(
  fixture: SwapCardStoryFixture,
): SwapCardStoryPresentation {
  switch (fixture) {
    case 'idle':
      return {
        walletConnected: false,
      };
    case 'quoting':
      return {
        walletConnected: true,
        seedFromAmount: '10',
        buttonState: 'refreshing_quote',
        quoteLoading: true,
        formattedRate: '1 XLM = 0.1050 USDC',
      };
    case 'stale':
      return {
        walletConnected: true,
        seedFromAmount: '10',
        buttonState: 'error',
        quoteStale: true,
        toAmount: '1.0500',
        formattedRate: '1 XLM = 0.1050 USDC',
        quotePriceImpact: 0.12,
      };
    case 'confirming':
      return {
        walletConnected: true,
        seedFromAmount: '10',
        buttonState: 'ready',
        toAmount: '1.0500',
        formattedRate: '1 XLM = 0.1050 USDC',
        quotePriceImpact: 0.12,
        confirmModalOpen: true,
        optimisticStatus: 'review',
        tradeParams: baseTradeParams,
      };
    case 'error':
      return {
        walletConnected: true,
        seedFromAmount: '10',
        buttonState: 'error',
        quoteError: {
          message:
            'Unable to fetch quote from SDEX. Check your connection and try again.',
        },
      };
    default:
      return { walletConnected: false };
  }
}
