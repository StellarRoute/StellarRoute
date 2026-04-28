import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useCompactMode } from './useCompactMode';

describe('useCompactMode', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('initializes with false by default', () => {
    const { result } = renderHook(() => useCompactMode());
    expect(result.current.isCompact).toBe(false);
  });

  it('loads saved preference from localStorage', () => {
    localStorage.setItem('stellarroute.compactMode', 'true');
    const { result } = renderHook(() => useCompactMode());
    expect(result.current.isCompact).toBe(true);
  });

  it('toggles compact mode', () => {
    const { result } = renderHook(() => useCompactMode());
    
    expect(result.current.isCompact).toBe(false);
    
    act(() => {
      result.current.toggleCompact();
    });
    
    expect(result.current.isCompact).toBe(true);
    
    act(() => {
      result.current.toggleCompact();
    });
    
    expect(result.current.isCompact).toBe(false);
  });

  it('persists state to localStorage', () => {
    const { result } = renderHook(() => useCompactMode());
    
    act(() => {
      result.current.setIsCompact(true);
    });
    
    expect(localStorage.getItem('stellarroute.compactMode')).toBe('true');
    
    act(() => {
      result.current.setIsCompact(false);
    });
    
    expect(localStorage.getItem('stellarroute.compactMode')).toBe('false');
  });

  it('handles localStorage errors gracefully', () => {
    const originalSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error('Storage full');
    };

    const { result } = renderHook(() => useCompactMode());
    
    // Should not throw
    act(() => {
      result.current.setIsCompact(true);
    });
    
    expect(result.current.isCompact).toBe(true);
    
    Storage.prototype.setItem = originalSetItem;
  });

  it('handles invalid localStorage data', () => {
    localStorage.setItem('stellarroute.compactMode', 'invalid');
    const { result } = renderHook(() => useCompactMode());
    expect(result.current.isCompact).toBe(false);
  });
});
