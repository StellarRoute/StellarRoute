import type { Story } from '@ladle/react';
import { useState } from 'react';
import { TransactionConfirmationModal } from './TransactionConfirmationModal';
import type { TransactionConfirmationModalProps } from './TransactionConfirmationModal';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';

// ── Shared mock quote ────────────────────────────────────────────────────────

const MOCK_WALLET =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

const baseTradeParams: TradeParams = {
  fromAsset: 'XLM',
  toAsset: 'USDC',
  fromAmount: '500.00',
  toAmount: '52.47',
  exchangeRate: '0.1049',
  priceImpact: '0.12',
  minReceived: '52.21 USDC',
  networkFee: '0.00001 XLM',
  routePath: [],
  walletAddress: MOCK_WALLET,
};

const splitRouteTradeParams: TradeParams = {
  fromAsset: 'XLM',
  toAsset: 'BTC',
  fromAmount: '10000.00',
  toAmount: '0.01662',
  exchangeRate: '0.000001662',
  priceImpact: '0.45',
  minReceived: '0.01645 BTC',
  networkFee: '0.00001 XLM',
  routePath: [],
  walletAddress: MOCK_WALLET,
};

const highSlippageTradeParams: TradeParams = {
  fromAsset: 'XLM',
  toAsset: 'AQUA',
  fromAmount: '50000.00',
  toAmount: '1750000.00',
  exchangeRate: '35.0',
  priceImpact: '8.5',
  minReceived: '1662500.00 AQUA',
  networkFee: '0.00001 XLM',
  routePath: [],
  walletAddress: MOCK_WALLET,
};

// ── Shared no-op handlers ────────────────────────────────────────────────────

const noop = () => {};
const noopAsync = async () => {};

function baseProps(
  overrides: Partial<TransactionConfirmationModalProps> = {},
): TransactionConfirmationModalProps {
  return {
    isOpen: true,
    status: 'review',
    tradeParams: baseTradeParams,
    onConfirm: noop,
    onCancel: noop,
    onTryAgain: noop,
    onResubmit: noop,
    onDismiss: noop,
    onDone: noop,
    ...overrides,
  };
}

// ── Stories ──────────────────────────────────────────────────────────────────

/** Review state — default XLM → USDC swap */
export const Default: Story = () => {
  const [open, setOpen] = useState(true);
  return (
    <TransactionConfirmationModal
      {...baseProps({ isOpen: open, onCancel: () => setOpen(false), onConfirm: () => setOpen(false) })}
    />
  );
};
Default.storyName = 'Default — Review';

/** Pending state — waiting for wallet signature */
export const Pending: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({ status: 'pending', tradeParams: undefined })}
  />
);
Pending.storyName = 'Pending — Wallet Signature';

/** Submitted state — awaiting network confirmation */
export const Submitted: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({ status: 'submitted', tradeParams: undefined })}
  />
);
Submitted.storyName = 'Submitted — Awaiting Network';

/** Confirmed state with tx hash */
export const Confirmed: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({
      status: 'confirmed',
      txHash: 'abc123def456abc123def456abc123def456abc123def456abc123def456ab12',
    })}
  />
);
Confirmed.storyName = 'Confirmed — Success';

/** Failed state with error message */
export const Failed: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({
      status: 'failed',
      tradeParams: undefined,
      errorMessage: 'Insufficient liquidity for this trade size. Try reducing the amount.',
    })}
  />
);
Failed.storyName = 'Failed — With Error';

/** Dropped / timed-out state */
export const Dropped: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({ status: 'dropped', tradeParams: undefined })}
  />
);
Dropped.storyName = 'Dropped — Timed Out';

/** High slippage warning — large AQUA trade with wide minReceived gap */
export const HighSlippageWarning: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({ tradeParams: highSlippageTradeParams })}
  />
);
HighSlippageWarning.storyName = 'High Slippage Warning';

/** Split route — multi-hop XLM → BTC */
export const SplitRoute: Story = () => (
  <TransactionConfirmationModal
    {...baseProps({ tradeParams: splitRouteTradeParams })}
  />
);
SplitRoute.storyName = 'Split Route — Multi-hop';

/** Mobile viewport at 390 px — wraps modal in a constrained container */
export const MobileViewport: Story = () => (
  <div style={{ width: 390, margin: '0 auto' }}>
    <TransactionConfirmationModal
      {...baseProps({ tradeParams: splitRouteTradeParams })}
    />
  </div>
);
MobileViewport.storyName = 'Mobile — 390px Viewport';
