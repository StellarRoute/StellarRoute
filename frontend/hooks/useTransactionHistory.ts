import { useState, useEffect } from 'react';
import { TransactionRecord, TransactionStatus } from '@/types/transaction';

const STORAGE_KEY = 'stellar_route_tx_history';

/**
 * Downgrades any `pending` or `submitted` records to `dropped`.
 *
 * Called when records are first loaded from localStorage (on page load or
 * wallet-address change) because in-flight state cannot be recovered after a
 * page reload.
 *
 * Property 2: Reload downgrades in-flight statuses to `dropped`
 * Validates: Requirements 6.3
 */
export function downgradePendingOnReload(
  records: TransactionRecord[]
): TransactionRecord[] {
  return records.map((tx) =>
    tx.status === 'pending' || tx.status === 'submitted'
      ? { ...tx, status: 'dropped' as TransactionStatus }
      : tx
  );
}

export function useTransactionHistory(walletAddress: string | null) {
  const [transactions, setTransactions] = useState<TransactionRecord[]>(() => {
    if (typeof window === 'undefined' || !walletAddress) return [];
    try {
      const stored = localStorage.getItem(`${STORAGE_KEY}_${walletAddress}`);
      const parsed: TransactionRecord[] = stored ? JSON.parse(stored) : [];
      return downgradePendingOnReload(parsed);
    } catch (e) {
      console.error('Failed to parse transaction history', e);
      return [];
    }
  });

  // Re-sync if walletAddress changes
  useEffect(() => {
    if (!walletAddress) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setTransactions([]);
      return;
    }
    try {
      const stored = localStorage.getItem(`${STORAGE_KEY}_${walletAddress}`);
      const parsed: TransactionRecord[] = stored ? JSON.parse(stored) : [];
      setTransactions(downgradePendingOnReload(parsed));
    } catch (e) {
      console.error('Failed to load transaction history', e);
      setTransactions([]);
    }
    // Note: This still has a setState, but standard for id change.
    // Suppressing the custom lint rule if necessary, or just leaving it.
  }, [walletAddress]);

  // Save to local storage whenever transactions change
  useEffect(() => {
    if (!walletAddress) return;
    
    try {
      localStorage.setItem(
        `${STORAGE_KEY}_${walletAddress}`,
        JSON.stringify(transactions)
      );
    } catch (e) {
      console.error('Failed to save transaction history to localStorage', e);
    }
  }, [transactions, walletAddress]);

  const addTransaction = (tx: TransactionRecord) => {
    setTransactions((prev) => [tx, ...prev]);
  };

  const updateTransactionStatus = (
    id: string,
    status: TransactionStatus,
    updates?: Partial<TransactionRecord>
  ) => {
    setTransactions((prev) =>
      prev.map((tx) =>
        tx.id === id ? { ...tx, status, ...updates } : tx
      )
    );
  };

  const clearHistory = () => {
    setTransactions([]);
    if (walletAddress) {
      localStorage.removeItem(`${STORAGE_KEY}_${walletAddress}`);
    }
  };

  return {
    transactions,
    addTransaction,
    updateTransactionStatus,
    clearHistory,
  };
}
