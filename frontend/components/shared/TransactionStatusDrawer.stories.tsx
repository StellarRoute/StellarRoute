import type { Story } from '@ladle/react';
import { useState } from 'react';

import { TransactionStatusDrawer } from './TransactionStatusDrawer';
import type { TransactionRecord } from '@/types/transaction';
import type { Asset, PathStep } from '@/types';

/**
 * Visual states for `TransactionStatusDrawer` (issue #1288).
 *
 * The drawer is a read-only view of a transaction record, so these stories only
 * supply fixtures for the pending / confirmed / failed states. No production
 * component is modified — `TransactionConfirmationModal` is deliberately left
 * out of scope.
 */

const MOCK_WALLET =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

const MOCK_HASH =
  'a3f9c2d4e5b6a7f8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

/** Fixed timestamp keeps the rendered date deterministic across snapshots. */
const MOCK_TIMESTAMP = 1713895200;

const XLM: Asset = { asset_type: 'native' };

const USDC: Asset = {
  asset_type: 'credit_alphanum4',
  asset_code: 'USDC',
  asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
};

const USDT: Asset = {
  asset_type: 'credit_alphanum4',
  asset_code: 'USDT',
  asset_issuer: 'GDUKMGUGDZQK6QHY5F3M6QVXKZRSYGT3D7IYZYVJZ7Z3QNZ5OZ3EUTYR',
};

const singleHopPath: PathStep[] = [
  {
    from_asset: XLM,
    to_asset: USDC,
    price: '0.1049',
    source: 'sdex',
    liquidity_depth: '125000',
    fee_bps: 0,
  },
];

const multiHopPath: PathStep[] = [
  {
    from_asset: XLM,
    to_asset: USDC,
    price: '0.1049',
    source: 'sdex',
    liquidity_depth: '125000',
    fee_bps: 0,
  },
  {
    from_asset: USDC,
    to_asset: USDT,
    price: '1.0001',
    source: 'amm:phoenix_pool_address',
    liquidity_depth: '98000',
    fee_bps: 30,
  },
];

function buildTransaction(
  overrides: Partial<TransactionRecord> = {}
): TransactionRecord {
  return {
    id: 'txn-abc123',
    timestamp: MOCK_TIMESTAMP,
    fromAsset: 'XLM',
    fromAmount: '500.00',
    toAsset: 'USDC',
    toAmount: '52.47',
    exchangeRate: '0.1049',
    priceImpact: '0.12',
    minReceived: '52.21',
    networkFee: '0.00001',
    routePath: singleHopPath,
    status: 'pending',
    walletAddress: MOCK_WALLET,
    ...overrides,
  };
}

/** Keeps the Radix sheet open so the drawer content is visible in the canvas. */
function OpenDrawer({ transaction }: { transaction: TransactionRecord }) {
  const [open, setOpen] = useState(true);
  return (
    <TransactionStatusDrawer
      transaction={transaction}
      isOpen={open}
      onOpenChange={setOpen}
    />
  );
}

/** Pending — transaction accepted locally, awaiting inclusion/finality. */
export const Pending: Story = () => (
  <OpenDrawer
    transaction={buildTransaction({
      status: 'pending',
      hash: MOCK_HASH,
      routePath: multiHopPath,
    })}
  />
);
Pending.storyName = 'Pending — Processing';

/** Confirmed — settled on-chain with a copyable hash and explorer links. */
export const Confirmed: Story = () => (
  <OpenDrawer
    transaction={buildTransaction({
      status: 'confirmed',
      hash: MOCK_HASH,
      routePath: multiHopPath,
    })}
  />
);
Confirmed.storyName = 'Confirmed — Success';

/** Failed — surfaces the error details block next to the swap summary. */
export const Failed: Story = () => (
  <OpenDrawer
    transaction={buildTransaction({
      status: 'failed',
      hash: MOCK_HASH,
      errorMessage:
        'Insufficient liquidity for this trade size. Try reducing the amount.',
      routePath: multiHopPath,
    })}
  />
);
Failed.storyName = 'Failed — With Error';
