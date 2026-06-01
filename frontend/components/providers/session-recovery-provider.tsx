import React, {
  ReactNode,
  createContext,
  useCallback,
  useContext,
  useMemo,
} from 'react';
import { useSessionRecovery } from '@/hooks/useSessionRecovery';
import {
  useFormStateRecovery,
  useQuoteStateRecovery,
} from '@/hooks/useFormStateRecovery';
import { useQuoteRefreshRecovery } from '@/hooks/useQuoteRefreshRecovery';

interface RecoveryData {
  baseAsset?: string;
  quoteAsset?: string;
  amount?: string;
}

interface SessionRecoveryContextType {
  // Session state
  isStale: boolean;
  isRecovering: boolean;
  refreshType: 'sleep' | 'refresh' | null;

  // Recovery actions
  beginRecovery: () => void;
  completeRecovery: () => void;
  dismissRecovery: () => void;

  // Form state management
  getSavedFormState: () => Record<string, any> | null;
  saveFormState: (state: Record<string, any>) => void;
  clearFormState: () => void;
  isFormStateValid: () => boolean;

  // Quote state management
  getSavedQuoteState: () => Record<string, any> | null;
  saveQuoteState: (state: Record<string, any>, ttl?: number) => void;
  clearQuoteState: () => void;
  isQuoteExpired: () => boolean;

  // Recovery helpers
  canProceedWithRecovery: () => boolean;
  getRecoveryData: () => RecoveryData | null;
  getParamsNeedingRefresh: () => RecoveryData | null;
}

const SessionRecoveryContext = createContext<
  SessionRecoveryContextType | undefined
>(undefined);

export function SessionRecoveryProvider({ children }: { children: ReactNode }) {
  const sessionRecovery = useSessionRecovery();
  const formStateRecovery = useFormStateRecovery();
  const quoteStateRecovery = useQuoteStateRecovery();
  const quoteRefreshRecovery = useQuoteRefreshRecovery();

  const value = useMemo<SessionRecoveryContextType>(
    () => ({
      // Session state
      isStale: sessionRecovery.isStale,
      isRecovering: sessionRecovery.isRecovering,
      refreshType: sessionRecovery.refreshType,

      // Recovery actions
      beginRecovery: sessionRecovery.beginRecovery,
      completeRecovery: sessionRecovery.completeRecovery,
      dismissRecovery: sessionRecovery.dismissRecovery,

      // Form state
      getSavedFormState: formStateRecovery.getSavedFormState,
      saveFormState: formStateRecovery.saveFormState,
      clearFormState: formStateRecovery.clearFormState,
      isFormStateValid: formStateRecovery.isFormStateValid,

      // Quote state
      getSavedQuoteState: quoteStateRecovery.getSavedQuoteState,
      saveQuoteState: quoteStateRecovery.saveQuoteState,
      clearQuoteState: quoteStateRecovery.clearQuoteState,
      isQuoteExpired: quoteStateRecovery.isQuoteExpired,

      // Recovery helpers
      canProceedWithRecovery: quoteRefreshRecovery.canProceedWithRecovery,
      getRecoveryData: quoteRefreshRecovery.getRecoveryData,
      getParamsNeedingRefresh: quoteRefreshRecovery.getParamsNeedingRefresh,
    }),
    [
      sessionRecovery,
      formStateRecovery,
      quoteStateRecovery,
      quoteRefreshRecovery,
    ]
  );

  return (
    <SessionRecoveryContext.Provider value={value}>
      {children}
    </SessionRecoveryContext.Provider>
  );
}

export function useSessionRecoveryContext(): SessionRecoveryContextType {
  const context = useContext(SessionRecoveryContext);
  if (!context) {
    throw new Error(
      'useSessionRecoveryContext must be used within SessionRecoveryProvider'
    );
  }
  return context;
}
