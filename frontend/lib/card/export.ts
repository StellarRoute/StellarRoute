/**
 * Client-side CSV export for virtual card transactions (CARD-51).
 *
 * Serializes card ledger activities (POS debits with merchant/fiat metadata
 * and CCTP top-up deposits) into RFC-4180 compliant CSV, entirely on the
 * client — no network request is made.
 *
 * This module is intentionally independent of the swap-history CSV exporter
 * (`lib/transaction-csv-export.ts`): card transactions have their own
 * columns and sign rules, and the two exporters must not share state.
 */

export type CardTransactionType = 'pos_debit' | 'cctp_topup';

/**
 * A single card ledger activity.
 *
 * Amounts are magnitudes as decimal strings; the export derives the sign
 * from `type` (POS debits are negative, CCTP top-ups are positive), so
 * callers never have to pre-sign amounts.
 */
export interface CardTransaction {
  /** ISO 8601 timestamp of the activity. */
  timestamp: string;

  /** Kind of activity. */
  type: CardTransactionType;

  /** Merchant name for POS debits; description for top-ups. */
  merchant: string;

  /** USDC amount as a decimal string (magnitude). */
  usdcAmount: string;

  /** Fiat amount as a decimal string (magnitude). */
  fiatAmount: string;

  /** Fiat currency code, e.g. `USD`. */
  currency: string;

  /** Transaction hash (Stellar transaction hash or authorization id). */
  txHash: string;
}

export const CARD_CSV_COLUMNS = [
  'Timestamp',
  'Type',
  'Description/Merchant',
  'USDC Amount',
  'Fiat Amount',
  'Currency',
  'Transaction Hash',
];

const TYPE_LABELS: Record<CardTransactionType, string> = {
  pos_debit: 'POS Debit',
  cctp_topup: 'CCTP Top-Up',
};

/**
 * Escape one CSV field per RFC 4180: fields containing a comma, a double
 * quote, or a line break are wrapped in double quotes, and embedded double
 * quotes are doubled.
 */
export function escapeCardCsvField(value: string): string {
  if (
    value.includes(',') ||
    value.includes('"') ||
    value.includes('\n') ||
    value.includes('\r')
  ) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

/**
 * Apply the sign rule for a card amount: POS debits are negative, CCTP
 * top-ups are positive. The input is treated as a magnitude, so a caller
 * passing an already-signed value still gets the correct sign. A zero
 * amount is never rendered as negative zero.
 */
function signedAmount(amount: string, type: CardTransactionType): string {
  const magnitude = amount.startsWith('-') ? amount.slice(1) : amount;
  if (type !== 'pos_debit') return magnitude;
  const numeric = Number(magnitude);
  if (Number.isNaN(numeric) || numeric === 0) return magnitude;
  return `-${magnitude}`;
}

export function exportCardTransactionsToCsv(
  transactions: CardTransaction[],
): string {
  const header = CARD_CSV_COLUMNS.map(escapeCardCsvField).join(',');
  const rows = transactions.map((tx) =>
    [
      tx.timestamp,
      TYPE_LABELS[tx.type],
      tx.merchant,
      signedAmount(tx.usdcAmount, tx.type),
      signedAmount(tx.fiatAmount, tx.type),
      tx.currency,
      tx.txHash,
    ]
      .map(escapeCardCsvField)
      .join(','),
  );
  return [header, ...rows].join('\n') + '\n';
}
