import type { Story } from '@ladle/react';
import { useState } from 'react';
import { TransactionStatusDrawer } from './TransactionStatusDrawer';
import type { TransactionRecord } from '@/types/transaction';
import type { PathStep } from '@/types';

// ── Shared mock transaction data ────────────────────────────────────────────

const MOCK_WALLET =
  'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

const MOCK_HASH =
  'a3f9c2d4e5b6a7f8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

const MOCK_TIMESTAMP = 1713895200; // fixed timestamp for deterministic renders

const singleHopPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    price: '0.1049',
    source: 'sdex',
  },
];

const multiHopPath: PathStep[] = [
  {
    from_asset: { asset_type: 'native' },
    to_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    price: '0.1049',
    source: 'sdex',
  },
  {
    from_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    },
    to_asset: {
      asset_type: 'credit_alphanum12',
      asset_code: 'USDT',
      asset_issuer: 'GDUKMGUGDZQK6QHY5F3M6QVXKZRSYGT3D7IYZYVJZ7Z3QNZ5OZ3EUTYR',
    },
    price: '1.0001',
    source: 'amm:phoenix_pool_address',
  },
];

function buildTransaction(
  overrides: Partial<TransactionRecord> = {},
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

// ── Shared open drawer wrapper ──────────────────────────────────────────────

interface WrapperProps {
  transaction: TransactionRecord;
}

function OpenDrawer({ transaction }: WrapperProps) {
  const [open, setOpen] = useState(true);
  return (
    <TransactionStatusDrawer
      transaction={transaction}
      isOpen={open}
      onOpenChange={(v) => setOpen(v)}
    />
  );
}

// ── Stories ─────────────────────────────────────────────────────────────────

/** Pending — transaction awaiting inclusion / finality */
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

/** Submitted — signed and submitted to network, before confirmation */
export const Submitted: Story = () => (
  <OpenDrawer
    transaction={buildTransaction({
      status: 'submitted',
      hash: MOCK_HASH,
    })}
  />
);
Submitted.storyName = 'Submitted — Awaiting Network';

/** Confirmed — successful transaction with a hash */
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

/** Failed — transaction failed with an error message */
export const Failed: Story = () => (
  <OpenDrawer
    transaction={buildTransaction({
      status: 'failed',
      errorMessage:
        'Insufficient liquidity for this trade size. Try reducing the amount.',
      routePath: multiHopPath,
    })}
  />
);
Failed.storyName = 'Failed — With Error';
