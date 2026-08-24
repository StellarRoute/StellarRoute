import { useCallback } from 'react';
import {
  useFormStateRecovery,
  useQuoteStateRecovery,
} from './useFormStateRecovery';

interface QuoteRefreshParams {
  baseAsset?: string;
  quoteAsset?: string;
  amount?: string;
}

export function useQuoteRefreshRecovery() {
  const { getSavedQuoteState, isQuoteExpired } = useQuoteStateRecovery();
  const { getSavedFormState } = useFormStateRecovery();

  // Get params that need quote refresh
  const getParamsNeedingRefresh = useCallback(() => {
    const quoteState = getSavedQuoteState();
    if (!quoteState) return null;

    // Check if quote is expired
    if (isQuoteExpired()) {
      return {
        baseAsset: quoteState.baseAsset,
        quoteAsset: quoteState.quoteAsset,
        amount: quoteState.amount,
      };
    }

    return null;
  }, [getSavedQuoteState, isQuoteExpired]);

  // Check if recovery can proceed (all required params present)
  const canProceedWithRecovery = useCallback(() => {
    const formState = getSavedFormState();
    if (!formState || !formState.baseAsset || !formState.quoteAsset) {
      return false;
    }
    return true;
  }, [getSavedFormState]);

  // Get recovery data (form state only, non-sensitive)
  const getRecoveryData = useCallback(() => {
    const formState = getSavedFormState();
    if (!formState) return null;

    // Return only non-sensitive form data
    return {
      baseAsset: formState.baseAsset,
      quoteAsset: formState.quoteAsset,
      amount: formState.amount,
    };
  }, [getSavedFormState]);

  return {
    getParamsNeedingRefresh,
    canProceedWithRecovery,
    getRecoveryData,
  };
}
