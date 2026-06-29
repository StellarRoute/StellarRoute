'use client';

import { useEffect, useRef, useState } from 'react';
import { useMotionTransition, MOTION_DURATION } from '@/lib/motion';

interface QuoteRefreshTransitionOptions {
  durationMs?: number;
}

interface QuoteRefreshTransitionState {
  isRefreshing: boolean;
  transitionStyle: React.CSSProperties;
}

/**
 * Hook to handle quote refresh animations.
 * Adds a subtle transition effect when quote values change.
 *
 * @param quoteKey - A key that changes when the quote updates (e.g., a hash or timestamp)
 * @param options - Configuration options
 * @returns State and styles for the refresh animation
 */
export function useQuoteRefreshTransition(
  quoteKey: string | number | undefined,
  { durationMs = MOTION_DURATION.QUOTE_REFRESH }: QuoteRefreshTransitionOptions = {},
): QuoteRefreshTransitionState {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const previousKeyRef = useRef(quoteKey);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const transition = useMotionTransition('all', durationMs);

  useEffect(() => {
    if (quoteKey !== previousKeyRef.current && previousKeyRef.current !== undefined) {
      setIsRefreshing(true);
      
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      
      timerRef.current = setTimeout(() => {
        setIsRefreshing(false);
      }, durationMs);
    }
    
    previousKeyRef.current = quoteKey;

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [quoteKey, durationMs]);

  return {
    isRefreshing,
    transitionStyle: {
      transition,
    },
  };
}
