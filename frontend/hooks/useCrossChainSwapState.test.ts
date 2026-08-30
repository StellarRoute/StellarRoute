import { renderHook, act } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useCrossChainSwapState } from './useCrossChainSwapState';
import { UNMATCHED_CORRIDOR_ID } from '@/lib/cross-chain/corridors';

describe('useCrossChainSwapState', () => {
  it('marks uncatalogued Sepolia → Bitcoin as unsupported', () => {
    const { result } = renderHook(() =>
      useCrossChainSwapState({
        initialSourceChainId: 'ethereum-sepolia',
        initialDestChainId: 'bitcoin',
      })
    );

    expect(result.current.isUncatalogued).toBe(true);
    expect(result.current.corridorId).toBe(UNMATCHED_CORRIDOR_ID);
    expect(result.current.executable).toBe(false);
    expect(result.current.canReview).toBe(false);
    expect(result.current.availability).toBe('unsupported');
  });

  it('keeps stellar-native executable without fallback on uncatalogued transitions', () => {
    const { result } = renderHook(() => useCrossChainSwapState());

    act(() => {
      result.current.selectSourceChain('ethereum-sepolia');
      result.current.selectDestChain('bitcoin');
    });

    expect(result.current.isUncatalogued).toBe(true);
    expect(result.current.executable).toBe(false);

    act(() => {
      result.current.selectCorridor('stellar-native');
    });

    expect(result.current.corridorId).toBe('stellar-native');
    expect(result.current.isStellarNativeExecutable).toBe(true);
  });

  it('syncs corridor tab selection with chain selectors', () => {
    const { result } = renderHook(() => useCrossChainSwapState());

    act(() => {
      result.current.selectCorridor('evm-to-stellar');
    });

    expect(result.current.sourceChainId).toBe('ethereum-sepolia');
    expect(result.current.destChainId).toBe('stellar');
    expect(result.current.corridorId).toBe('evm-to-stellar');
    expect(result.current.executable).toBe(false);
  });

  it('marks default Stellar → Sepolia as catalog-executable', () => {
    const { result } = renderHook(() => useCrossChainSwapState());

    expect(result.current.sourceChainId).toBe('stellar');
    expect(result.current.destChainId).toBe('ethereum-sepolia');
    expect(result.current.corridorId).toBe('stellar-to-evm');
    expect(result.current.executable).toBe(true);
    expect(result.current.isStellarNativeExecutable).toBe(false);
  });

  it('does not allow review for catalogued coming-soon corridors', () => {
    const { result } = renderHook(() =>
      useCrossChainSwapState({
        initialSourceChainId: 'ethereum-sepolia',
        initialDestChainId: 'stellar',
      })
    );

    expect(result.current.canReview).toBe(false);
  });
});
