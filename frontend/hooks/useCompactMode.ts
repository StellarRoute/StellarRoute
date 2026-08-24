import { useEffect, useState } from 'react';

const COMPACT_MODE_KEY = 'stellarroute.compactMode';

/**
 * Hook for managing compact layout mode preference
 * Persists toggle state in localStorage
 */
export function useCompactMode() {
  const [isCompact, setIsCompact] = useState<boolean>(() => {
    if (typeof window === 'undefined') return false;
    
    try {
      const stored = localStorage.getItem(COMPACT_MODE_KEY);
      return stored === 'true';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(COMPACT_MODE_KEY, String(isCompact));
    } catch (error) {
      console.error('Failed to save compact mode preference', error);
    }
  }, [isCompact]);

  const toggleCompact = () => setIsCompact((prev) => !prev);

  return {
    isCompact,
    setIsCompact,
    toggleCompact,
  };
}
