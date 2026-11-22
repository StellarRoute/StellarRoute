'use client';

import {
  exportCardTransactionsToCsv,
  type CardTransaction,
} from '@/lib/card/export';

export interface CardExportButtonProps {
  transactions: CardTransaction[];
}

/**
 * Client-side CSV export button for the card deck (CARD-51).
 *
 * Builds the CSV in the browser and triggers a Blob download named
 * `stellarroute-card-activity-<YYYY-MM-DD>.csv`. No network request is
 * made. The button is disabled while the card transaction list is empty.
 */
export function CardExportButton({ transactions }: CardExportButtonProps) {
  const isEmpty = transactions.length === 0;

  const handleClick = () => {
    if (isEmpty) return;
    const csv = exportCardTransactionsToCsv(transactions);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.setAttribute('href', url);
    link.setAttribute(
      'download',
      `stellarroute-card-activity-${new Date().toISOString().slice(0, 10)}.csv`,
    );
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  return (
    <button
      type="button"
      data-testid="card-export-button"
      disabled={isEmpty}
      onClick={handleClick}
      className="rounded-lg px-4 py-2 font-medium disabled:opacity-60"
    >
      Export Activity (CSV)
    </button>
  );
}
