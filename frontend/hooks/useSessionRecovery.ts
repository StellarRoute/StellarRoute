import { useCallback, useEffect, useRef, useState } from 'react';

interface SessionCheckpoint {
  timestamp: number;
  isRefresh: boolean;
}

interface RecoveryState {
  isStale: boolean;
  isRecovering: boolean;
  refreshType: 'sleep' | 'refresh' | null;
  hasRecoverableContext: boolean;
}

const SESSION_CHECKPOINT_KEY = 'stellar_session_checkpoint';
const SESSION_THRESHOLD_MS = 30000; // 30 seconds threshold for detecting sleep/wake

export function useSessionRecovery() {
  const [state, setState] = useState<RecoveryState>({
    isStale: false,
    isRecovering: false,
    refreshType: null,
    hasRecoverableContext: false,
  });

  const lastCheckpointRef = useRef<SessionCheckpoint | null>(null);
  const visibilityHandlerRef = useRef<(() => void) | null>(null);
  const pageShowHandlerRef = useRef<((e: PageTransitionEvent) => void) | null>(null);

  // Update checkpoint
  const updateCheckpoint = useCallback((isRefresh = false) => {
    const checkpoint: SessionCheckpoint = {
      timestamp: Date.now(),
      isRefresh,
    };
    lastCheckpointRef.current = checkpoint;
    sessionStorage.setItem(SESSION_CHECKPOINT_KEY, JSON.stringify(checkpoint));
  }, []);

  // Check if session is stale based on time gap
  const checkSessionFreshness = useCallback(() => {
    if (!lastCheckpointRef.current) return null;

    const now = Date.now();
    const timeSinceCheckpoint = now - lastCheckpointRef.current.timestamp;

    if (timeSinceCheckpoint > SESSION_THRESHOLD_MS) {
      const refreshType = lastCheckpointRef.current.isRefresh ? 'refresh' : 'sleep';
      return refreshType;
    }

    return null;
  }, []);

  // Initialize checkpoint from session storage
  useEffect(() => {
    const stored = sessionStorage.getItem(SESSION_CHECKPOINT_KEY);
    if (stored) {
      try {
        lastCheckpointRef.current = JSON.parse(stored);
      } catch {
        lastCheckpointRef.current = null;
      }
    }
    
    // Set initial checkpoint if none exists
    if (!lastCheckpointRef.current) {
      updateCheckpoint(false);
    }
  }, []);

  // Check if there's recoverable context in storage
  const checkRecoverableContext = useCallback(() => {
    try {
      const formData = localStorage.getItem('stellar-route-trade-form');
      if (!formData) return false;
      
      const parsed = JSON.parse(formData);
      return !!(parsed && (parsed.amount || parsed.fromToken || parsed.toToken));
    } catch {
      return false;
    }
  }, []);

  // Handle page visibility change (tab sleep/wake)
  useEffect(() => {
    visibilityHandlerRef.current = () => {
      if (document.visibilityState === 'visible') {
        const refreshType = checkSessionFreshness();
        const hasRecoverableContext = checkRecoverableContext();
        
        if (refreshType && hasRecoverableContext) {
          setState({
            isStale: true,
            isRecovering: false,
            refreshType,
            hasRecoverableContext: true,
          });
        }
      }
      updateCheckpoint(false);
    };

    document.addEventListener('visibilitychange', visibilityHandlerRef.current);
    return () => {
      if (visibilityHandlerRef.current) {
        document.removeEventListener('visibilitychange', visibilityHandlerRef.current);
      }
    };
  }, [checkSessionFreshness, updateCheckpoint, checkRecoverableContext]);

  // Handle page show event (tab refresh/navigation back)
  useEffect(() => {
    pageShowHandlerRef.current = (event: PageTransitionEvent) => {
      if (event.persisted === false) {
        const refreshType = checkSessionFreshness();
        const hasRecoverableContext = checkRecoverableContext();
        
        if (refreshType && hasRecoverableContext) {
          setState({
            isStale: true,
            isRecovering: false,
            refreshType: 'refresh',
            hasRecoverableContext: true,
          });
        }
      }
      updateCheckpoint(true);
    };

    window.addEventListener('pageshow', pageShowHandlerRef.current);
    return () => {
      if (pageShowHandlerRef.current) {
        window.removeEventListener('pageshow', pageShowHandlerRef.current);
      }
    };
  }, [checkSessionFreshness, updateCheckpoint, checkRecoverableContext]);

  // Update checkpoint on regular interval
  useEffect(() => {
    const interval = setInterval(() => {
      updateCheckpoint(false);
    }, 5000);

    return () => clearInterval(interval);
  }, [updateCheckpoint]);

  // Begin recovery process
  const beginRecovery = useCallback(() => {
    setState((prev) => ({
      ...prev,
      isRecovering: true,
    }));
  }, []);

  // Complete recovery (restore)
  const completeRecovery = useCallback(() => {
    setState({
      isStale: false,
      isRecovering: false,
      refreshType: null,
      hasRecoverableContext: false,
    });
    updateCheckpoint(false);
  }, [updateCheckpoint]);

  // Dismiss recovery (reset)
  const dismissRecovery = useCallback(() => {
    setState({
      isStale: false,
      isRecovering: false,
      refreshType: null,
      hasRecoverableContext: false,
    });
    updateCheckpoint(false);
  }, [updateCheckpoint]);

  return {
    ...state,
    beginRecovery,
    completeRecovery,
    dismissRecovery,
  };
}
