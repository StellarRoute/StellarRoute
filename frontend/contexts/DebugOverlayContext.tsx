'use client';

import { createContext, useContext, useState, ReactNode } from 'react';
import type { DebugInfo } from '@/components/debug/DebugOverlay';

interface DebugOverlayContextType {
  debugInfo: DebugInfo;
  setDebugInfo: (info: DebugInfo) => void;
}

const DebugOverlayContext = createContext<DebugOverlayContextType | undefined>(undefined);

export function DebugOverlayProvider({ children }: { children: ReactNode }) {
  const [debugInfo, setDebugInfo] = useState<DebugInfo>({});

  return (
    <DebugOverlayContext.Provider value={{ debugInfo, setDebugInfo }}>
      {children}
    </DebugOverlayContext.Provider>
  );
}

export function useDebugOverlay() {
  const context = useContext(DebugOverlayContext);
  if (!context) {
    throw new Error('useDebugOverlay must be used within DebugOverlayProvider');
  }
  return context;
}
