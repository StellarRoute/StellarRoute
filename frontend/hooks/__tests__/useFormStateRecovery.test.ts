import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import {
  useFormStateRecovery,
  useQuoteStateRecovery,
} from '@/hooks/useFormStateRecovery';

describe('useFormStateRecovery', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  afterEach(() => {
    sessionStorage.clear();
  });

  it('should save and retrieve form state', () => {
    const { result } = renderHook(() => useFormStateRecovery());

    const formState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
      amount: '100',
    };

    act(() => {
      result.current.saveFormState(formState);
    });

    const saved = result.current.getSavedFormState();
    expect(saved).toEqual(expect.objectContaining(formState));
    expect(saved?.timestamp).toBeTruthy();
  });

  it('should clear form state', () => {
    const { result } = renderHook(() => useFormStateRecovery());

    const formState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
      amount: '100',
    };

    act(() => {
      result.current.saveFormState(formState);
    });

    expect(result.current.getSavedFormState()).toBeTruthy();

    act(() => {
      result.current.clearFormState();
    });

    expect(result.current.getSavedFormState()).toBeNull();
  });

  it('should return null for expired form state', () => {
    const { result } = renderHook(() => useFormStateRecovery());

    const formState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
      amount: '100',
    };

    act(() => {
      result.current.saveFormState(formState);
    });

    // Manually set expired timestamp
    const stored = sessionStorage.getItem('stellar_form_state');
    if (stored) {
      const parsed = JSON.parse(stored);
      parsed.timestamp = Date.now() - 6 * 60 * 1000; // 6 minutes ago
      sessionStorage.setItem('stellar_form_state', JSON.stringify(parsed));
    }

    // Create new hook instance to bypass ref cache
    const { result: result2 } = renderHook(() => useFormStateRecovery());

    expect(result2.current.getSavedFormState()).toBeNull();
  });

  it('should validate form state validity', () => {
    const { result } = renderHook(() => useFormStateRecovery());

    expect(result.current.isFormStateValid()).toBe(false);

    act(() => {
      result.current.saveFormState({
        baseAsset: 'native',
        quoteAsset: 'USDC',
      });
    });

    expect(result.current.isFormStateValid()).toBe(true);
  });
});

describe('useQuoteStateRecovery', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  afterEach(() => {
    sessionStorage.clear();
  });

  it('should save and retrieve quote state', () => {
    const { result } = renderHook(() => useQuoteStateRecovery());

    const quoteState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
      amount: '100',
    };

    act(() => {
      result.current.saveQuoteState(quoteState);
    });

    const saved = result.current.getSavedQuoteState();
    expect(saved).toEqual(expect.objectContaining(quoteState));
  });

  it('should use default TTL when not specified', () => {
    const { result } = renderHook(() => useQuoteStateRecovery());

    const quoteState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
    };

    act(() => {
      result.current.saveQuoteState(quoteState);
    });

    const saved = result.current.getSavedQuoteState();
    expect(saved?.ttl).toBe(2 * 60 * 1000); // Default 2 minutes
  });

  it('should use custom TTL when specified', () => {
    const { result } = renderHook(() => useQuoteStateRecovery());

    const quoteState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
    };

    const customTtl = 60000; // 1 minute

    act(() => {
      result.current.saveQuoteState(quoteState, customTtl);
    });

    const saved = result.current.getSavedQuoteState();
    expect(saved?.ttl).toBe(customTtl);
  });

  it('should detect expired quotes', () => {
    const { result } = renderHook(() => useQuoteStateRecovery());

    const quoteState = {
      baseAsset: 'native',
      quoteAsset: 'USDC',
    };

    act(() => {
      result.current.saveQuoteState(quoteState);
    });

    expect(result.current.isQuoteExpired()).toBe(false);

    // Manually set expired timestamp
    const stored = sessionStorage.getItem('stellar_quote_state');
    if (stored) {
      const parsed = JSON.parse(stored);
      parsed.timestamp = Date.now() - 3 * 60 * 1000; // 3 minutes ago
      sessionStorage.setItem('stellar_quote_state', JSON.stringify(parsed));
    }

    // Create new hook instance to bypass ref cache
    const { result: result2 } = renderHook(() => useQuoteStateRecovery());

    expect(result2.current.isQuoteExpired()).toBe(true);
  });

  it('should clear quote state', () => {
    const { result } = renderHook(() => useQuoteStateRecovery());

    act(() => {
      result.current.saveQuoteState({ baseAsset: 'native' });
    });

    expect(result.current.getSavedQuoteState()).toBeTruthy();

    act(() => {
      result.current.clearQuoteState();
    });

    expect(result.current.getSavedQuoteState()).toBeNull();
  });
});
