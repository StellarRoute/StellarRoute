import { useCallback, useRef } from 'react';

interface FormCheckpoint {
  baseAsset?: string;
  quoteAsset?: string;
  amount?: string;
  timestamp: number;
}

interface QuoteCheckpoint {
  baseAsset?: string;
  quoteAsset?: string;
  amount?: string;
  timestamp: number;
  ttl?: number;
}

const QUOTE_STATE_KEY = 'stellar_quote_state';
const FORM_STATE_TTL = 5 * 60 * 1000; // 5 minutes
const QUOTE_STATE_TTL = 2 * 60 * 1000; // 2 minutes

export function useFormStateRecovery() {
  const stateRef = useRef<FormCheckpoint | null>(null);

  const getSavedFormState = useCallback((): FormCheckpoint | null => {
    if (stateRef.current) return stateRef.current;

    try {
      const stored = localStorage.getItem('stellar-route-trade-form');
      if (!stored) return null;

      const parsed = JSON.parse(stored);
      if (!parsed) return null;

      const checkpoint: FormCheckpoint = {
        baseAsset: parsed.fromToken,
        quoteAsset: parsed.toToken,
        amount: parsed.amount,
        timestamp: parsed.savedAt || parsed.timestamp || Date.now(),
      };

      const isExpired = Date.now() - checkpoint.timestamp > FORM_STATE_TTL;

      if (isExpired) {
        localStorage.removeItem('stellar-route-trade-form');
        return null;
      }

      stateRef.current = checkpoint;
      return checkpoint;
    } catch {
      return null;
    }
  }, []);

  const saveFormState = useCallback(
    (state: Omit<FormCheckpoint, 'timestamp'>) => {
      const checkpoint = {
        fromToken: state.baseAsset,
        toToken: state.quoteAsset,
        amount: state.amount,
        savedAt: Date.now(),
        slippage: 0.5,
        deadline: 30,
        side: 'sell',
      };
      stateRef.current = {
        baseAsset: state.baseAsset,
        quoteAsset: state.quoteAsset,
        amount: state.amount,
        timestamp: checkpoint.savedAt,
      };
      localStorage.setItem('stellar-route-trade-form', JSON.stringify(checkpoint));
    },
    []
  );

  const clearFormState = useCallback(() => {
    stateRef.current = null;
    localStorage.removeItem('stellar-route-trade-form');
  }, []);

  const isFormStateValid = useCallback(() => {
    const saved = getSavedFormState();
    if (!saved) return false;
    return Date.now() - saved.timestamp < FORM_STATE_TTL;
  }, [getSavedFormState]);

  return {
    getSavedFormState,
    saveFormState,
    clearFormState,
    isFormStateValid,
  };
}

export function useQuoteStateRecovery() {
  const stateRef = useRef<QuoteCheckpoint | null>(null);

  const getSavedQuoteState = useCallback((): QuoteCheckpoint | null => {
    if (stateRef.current) return stateRef.current;

    try {
      const stored = sessionStorage.getItem(QUOTE_STATE_KEY);
      if (!stored) return null;

      const parsed = JSON.parse(stored) as QuoteCheckpoint;
      const ttl = parsed.ttl || QUOTE_STATE_TTL;
      const isExpired = Date.now() - parsed.timestamp > ttl;

      if (isExpired) {
        sessionStorage.removeItem(QUOTE_STATE_KEY);
        return null;
      }

      stateRef.current = parsed;
      return parsed;
    } catch {
      return null;
    }
  }, []);

  const saveQuoteState = useCallback(
    (state: Omit<QuoteCheckpoint, 'timestamp'>, ttl?: number) => {
      const checkpoint: QuoteCheckpoint = {
        ...state,
        timestamp: Date.now(),
        ttl: ttl || QUOTE_STATE_TTL,
      };
      stateRef.current = checkpoint;
      sessionStorage.setItem(QUOTE_STATE_KEY, JSON.stringify(checkpoint));
    },
    []
  );

  const clearQuoteState = useCallback(() => {
    stateRef.current = null;
    sessionStorage.removeItem(QUOTE_STATE_KEY);
  }, []);

  const isQuoteExpired = useCallback(() => {
    const saved = getSavedQuoteState();
    if (!saved) return true;
    const ttl = saved.ttl || QUOTE_STATE_TTL;
    return Date.now() - saved.timestamp > ttl;
  }, [getSavedQuoteState]);

  return {
    getSavedQuoteState,
    saveQuoteState,
    clearQuoteState,
    isQuoteExpired,
  };
}
