/**
 * CARD-44: Unified card activity feed types + mock normalization.
 * Distinguishes POS debits vs CCTP top-up deposits.
 */

export type CardActivityType = 'pos_debit' | 'cctp_deposit';

export type EvmChainId = 'ethereum' | 'base' | 'arbitrum' | 'optimism' | 'polygon';

export interface CardActivityBase {
  id: string;
  type: CardActivityType;
  /** ISO timestamp */
  createdAt: string;
  /** Signed USDC amount: negative for debit, positive for deposit */
  usdcAmount: number;
}

export interface PosDebitActivity extends CardActivityBase {
  type: 'pos_debit';
  merchantName: string;
  fiatAmount: number;
  fiatCurrency: string;
}

export interface CctpDepositActivity extends CardActivityBase {
  type: 'cctp_deposit';
  sourceChain: EvmChainId;
  /** Stellar transaction hash (for Stellar Expert) */
  stellarTxHash?: string;
  /** Optional EVM tx hash (for Etherscan-style explorers) */
  evmTxHash?: string;
}

export type CardActivityItem = PosDebitActivity | CctpDepositActivity;

/** Raw mock shape that may come from fixtures / API mocks */
export interface RawCardActivity {
  id: string;
  type: string;
  createdAt: string;
  usdcAmount: number;
  merchantName?: string;
  fiatAmount?: number;
  fiatCurrency?: string;
  sourceChain?: string;
  stellarTxHash?: string;
  evmTxHash?: string;
}

const EVM_CHAINS: EvmChainId[] = [
  'ethereum',
  'base',
  'arbitrum',
  'optimism',
  'polygon',
];

function isEvmChain(value: string | undefined): value is EvmChainId {
  return !!value && (EVM_CHAINS as string[]).includes(value);
}

/**
 * Normalize raw mock/API rows into typed activity items.
 * Unknown types are skipped.
 */
export function normalizeCardActivity(
  raw: RawCardActivity[],
): CardActivityItem[] {
  const items: CardActivityItem[] = [];

  for (const row of raw) {
    if (row.type === 'pos_debit') {
      items.push({
        id: row.id,
        type: 'pos_debit',
        createdAt: row.createdAt,
        usdcAmount: row.usdcAmount,
        merchantName: row.merchantName ?? 'Merchant',
        fiatAmount: row.fiatAmount ?? 0,
        fiatCurrency: row.fiatCurrency ?? 'USD',
      });
    } else if (row.type === 'cctp_deposit') {
      items.push({
        id: row.id,
        type: 'cctp_deposit',
        createdAt: row.createdAt,
        usdcAmount: row.usdcAmount,
        sourceChain: isEvmChain(row.sourceChain) ? row.sourceChain : 'ethereum',
        stellarTxHash: row.stellarTxHash,
        evmTxHash: row.evmTxHash,
      });
    }
  }

  return sortActivityNewestFirst(items);
}

/** Most recent first */
export function sortActivityNewestFirst(
  items: CardActivityItem[],
): CardActivityItem[] {
  return [...items].sort(
    (a, b) =>
      new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
  );
}

export function stellarExpertUrl(txHash: string): string {
  return `https://stellar.expert/explorer/public/tx/${txHash}`;
}

export function evmExplorerUrl(chain: EvmChainId, txHash: string): string {
  const hosts: Record<EvmChainId, string> = {
    ethereum: 'https://etherscan.io/tx/',
    base: 'https://basescan.org/tx/',
    arbitrum: 'https://arbiscan.io/tx/',
    optimism: 'https://optimistic.etherscan.io/tx/',
    polygon: 'https://polygonscan.com/tx/',
  };
  return `${hosts[chain]}${txHash}`;
}

export function formatUsdcSigned(amount: number): string {
  const abs = Math.abs(amount).toFixed(2);
  if (amount > 0) return `+${abs} USDC`;
  if (amount < 0) return `-${abs} USDC`;
  return `${abs} USDC`;
}