import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSessionRecovery } from '@/hooks/useSessionRecovery';

describe('useSessionRecovery', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    sessionStorage.clear();
  });

  describe('initialization', () => {
    it('should initialize with stale state false', () => {
      const { result } = renderHook(() => useSessionRecovery());
      expect(result.current.isStale).toBe(false);
      expect(result.current.isRecovering).toBe(false);
      expect(result.current.refreshType).toBeNull();
      expect(result.current.hasRecoverableContext).toBe(false);
    });
  });

  describe('sleep/wake detection', () => {
    it('should detect tab wake after 30+ seconds with recoverable context', () => {
      // Set up recoverable context in localStorage
      localStorage.setItem('stellar-route-trade-form', JSON.stringify({
        amount: '100',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now()
      }));

      const { result } = renderHook(() => useSessionRecovery());

      // Simulate initial checkpoint
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      // Simulate tab going to sleep and waking up after 30+ seconds
      act(() => {
        vi.advanceTimersByTime(35000);
        document.dispatchEvent(new Event('visibilitychange'));
        Object.defineProperty(document, 'visibilityState', {
          value: 'visible',
          writable: true,
        });
      });

      // Check stale state was set
      expect(result.current.isStale).toBe(true);
      expect(result.current.refreshType).toBe('sleep');
      expect(result.current.hasRecoverableContext).toBe(true);
    });

    it('should not detect as stale without recoverable context', () => {
      localStorage.clear(); // No recoverable context

      const { result } = renderHook(() => useSessionRecovery());

      // Simulate tab going to sleep and waking up after 30+ seconds
      act(() => {
        vi.advanceTimersByTime(35000);
        document.dispatchEvent(new Event('visibilitychange'));
        Object.defineProperty(document, 'visibilityState', {
          value: 'visible',
          writable: true,
        });
      });

      // Should not be stale without recoverable context
      expect(result.current.isStale).toBe(false);
    });

    it('should not detect as stale for brief visibility changes', () => {
      const { result } = renderHook(() => useSessionRecovery());

      // Simulate brief visibility change
      act(() => {
        vi.advanceTimersByTime(5000); // Only 5 seconds
        document.dispatchEvent(new Event('visibilitychange'));
        Object.defineProperty(document, 'visibilityState', {
          value: 'visible',
          writable: true,
        });
      });

      // Should not be stale
      expect(result.current.isStale).toBe(false);
    });
  });

  describe('recovery actions', () => {
    it('should transition to recovering state', () => {
      const { result } = renderHook(() => useSessionRecovery());

      act(() => {
        result.current.beginRecovery();
      });

      expect(result.current.isRecovering).toBe(true);
    });

    it('should detect refresh with recoverable context', () => {
      // Set up recoverable context
      localStorage.setItem('stellar-route-trade-form', JSON.stringify({
        amount: '100',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now()
      }));

      const { result } = renderHook(() => useSessionRecovery());

      // Simulate page refresh after delay
      act(() => {
        vi.advanceTimersByTime(35000);
        const event = new PageTransitionEvent('pageshow', { persisted: false });
        window.dispatchEvent(event);
      });

      expect(result.current.isStale).toBe(true);
      expect(result.current.refreshType).toBe('refresh');
      expect(result.current.hasRecoverableContext).toBe(true);
    });

    it('should complete recovery and clear stale state', () => {
      // Set up recoverable context
      localStorage.setItem('stellar-route-trade-form', JSON.stringify({
        amount: '100',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now()
      }));

      const { result } = renderHook(() => useSessionRecovery());

      // Set up stale state
      act(() => {
        vi.advanceTimersByTime(35000);
        document.dispatchEvent(new Event('visibilitychange'));
        Object.defineProperty(document, 'visibilityState', {
          value: 'visible',
          writable: true,
        });
      });

      expect(result.current.isStale).toBe(true);

      // Complete recovery
      act(() => {
        result.current.completeRecovery();
      });

      expect(result.current.isStale).toBe(false);
      expect(result.current.isRecovering).toBe(false);
      expect(result.current.refreshType).toBeNull();
      expect(result.current.hasRecoverableContext).toBe(false);
    });

    it('should dismiss recovery without restoring', () => {
      const { result } = renderHook(() => useSessionRecovery());

      // Set up stale state
      act(() => {
        vi.advanceTimersByTime(35000);
        document.dispatchEvent(new Event('visibilitychange'));
        Object.defineProperty(document, 'visibilityState', {
          value: 'visible',
          writable: true,
        });
      });

      expect(result.current.isStale).toBe(true);

      // Dismiss
      act(() => {
        result.current.dismissRecovery();
      });

      expect(result.current.isStale).toBe(false);
    });
  });

  describe('checkpoint updates', () => {
    it('should update checkpoint on regular intervals', () => {
      const getCheckpoint = () => {
        const stored = sessionStorage.getItem('stellar_session_checkpoint');
        return stored ? JSON.parse(stored) : null;
      };

      const { result } = renderHook(() => useSessionRecovery());

      // Initial checkpoint should be created after first interval
      act(() => {
        vi.advanceTimersByTime(5000);
      });

      const checkpoint1 = getCheckpoint();
      expect(checkpoint1).toBeTruthy();
      expect(checkpoint1.timestamp).toBeTruthy();

      // Update after another interval
      act(() => {
        vi.advanceTimersByTime(5000);
      });

      const checkpoint2 = getCheckpoint();
      expect(checkpoint2.timestamp).toBeGreaterThan(checkpoint1.timestamp);
    });
  });
});
