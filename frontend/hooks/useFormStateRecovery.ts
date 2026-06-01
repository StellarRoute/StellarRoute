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

const FORM_STATE_KEY = 'stellar_form_state';
const QUOTE_STATE_KEY = 'stellar_quote_state';
const FORM_STATE_TTL = 5 * 60 * 1000; // 5 minutes
const QUOTE_STATE_TTL = 2 * 60 * 1000; // 2 minutes

export function useFormStateRecovery() {
  const stateRef = useRef<FormCheckpoint | null>(null);

  const getSavedFormState = useCallback((): FormCheckpoint | null => {
    if (stateRef.current) return stateRef.current;

    try {
      const stored = sessionStorage.getItem(FORM_STATE_KEY);
      if (!stored) return null;

      const parsed = JSON.parse(stored) as FormCheckpoint;
      const isExpired = Date.now() - parsed.timestamp > FORM_STATE_TTL;

      if (isExpired) {
        sessionStorage.removeItem(FORM_STATE_KEY);
        return null;
      }

      stateRef.current = parsed;
      return parsed;
    } catch {
      return null;
    }
  }, []);

  const saveFormState = useCallback(
    (state: Omit<FormCheckpoint, 'timestamp'>) => {
      const checkpoint: FormCheckpoint = {
        ...state,
        timestamp: Date.now(),
      };
      stateRef.current = checkpoint;
      sessionStorage.setItem(FORM_STATE_KEY, JSON.stringify(checkpoint));
    },
    []
  );

  const clearFormState = useCallback(() => {
    stateRef.current = null;
    sessionStorage.removeItem(FORM_STATE_KEY);
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
