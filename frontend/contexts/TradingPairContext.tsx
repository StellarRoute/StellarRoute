'use client';

import { createContext, useContext, useState, ReactNode, useCallback } from 'react';

interface TradingPairContextType {
  fromAsset: string | null;
  toAsset: string | null;
  setTradingPair: (from: string, to: string) => void;
  clearTradingPair: () => void;
}

const TradingPairContext = createContext<TradingPairContextType | undefined>(undefined);

export function TradingPairProvider({ children }: { children: ReactNode }) {
  const [fromAsset, setFromAsset] = useState<string | null>(null);
  const [toAsset, setToAsset] = useState<string | null>(null);

  const setTradingPair = useCallback((from: string, to: string) => {
    setFromAsset(from);
    setToAsset(to);
  }, []);

  const clearTradingPair = useCallback(() => {
    setFromAsset(null);
    setToAsset(null);
  }, []);

  return (
    <TradingPairContext.Provider
      value={{
        fromAsset,
        toAsset,
        setTradingPair,
        clearTradingPair,
      }}
    >
      {children}
    </TradingPairContext.Provider>
  );
}

export function useTradingPair() {
  const context = useContext(TradingPairContext);
  if (context === undefined) {
    throw new Error('useTradingPair must be used within a TradingPairProvider');
  }
  return context;
}

export function useOptionalTradingPair(): TradingPairContextType | undefined {
  return useContext(TradingPairContext);
}
