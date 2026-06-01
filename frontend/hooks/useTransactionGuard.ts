"use client";

import * as React from "react";
import { useWallet } from "@/components/providers/wallet-provider";

export function useTransactionGuard() {
  const { isTransactionPending, setTransactionPending } = useWallet();

  const startTransaction = React.useCallback(() => {
    setTransactionPending(true);
  }, [setTransactionPending]);

  const endTransaction = React.useCallback(() => {
    setTransactionPending(false);
  }, [setTransactionPending]);

  const withTransactionGuard = React.useCallback(
    async <T>(fn: () => Promise<T>): Promise<T> => {
      startTransaction();
      try {
        return await fn();
      } finally {
        endTransaction();
      }
    },
    [startTransaction, endTransaction]
  );

  return {
    isTransactionPending,
    startTransaction,
    endTransaction,
    withTransactionGuard,
  };
}
