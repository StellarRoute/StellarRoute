import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useCardFreezeAction } from './useCardFreezeAction';

describe('useCardFreezeAction (CARD-50)', () => {
  it('progresses freeze lifecycle and sets frozen', async () => {
    const { result } = renderHook(() =>
      useCardFreezeAction({
        requestSignature: async () => ({
          ok: true,
          txHash: 'TX-FREEZE',
        }),
      }),
    );

    expect(result.current.lifecycle).toBe('idle');
    expect(result.current.isFrozen).toBe(false);

    await act(async () => {
      await result.current.freeze();
    });

    expect(result.current.isFrozen).toBe(true);
    expect(result.current.lastTxHash).toBe('TX-FREEZE');
    expect(result.current.lifecycle).toBe('idle');
  });

  it('handles wallet rejection without crashing', async () => {
    const onRejected = vi.fn();
    const { result } = renderHook(() =>
      useCardFreezeAction({
        onRejected,
        requestSignature: async () => ({
          ok: false,
          rejected: true,
        }),
      }),
    );

    await act(async () => {
      await result.current.freeze();
    });

    expect(result.current.isFrozen).toBe(false);
    expect(result.current.lifecycle).toBe('idle');
    expect(onRejected).toHaveBeenCalled();
  });

  it('unfreeze toggles state back', async () => {
    const { result } = renderHook(() =>
      useCardFreezeAction({
        requestSignature: async (action) => ({
          ok: true,
          txHash: `TX-${action}`,
        }),
      }),
    );

    await act(async () => {
      await result.current.freeze();
    });
    expect(result.current.isFrozen).toBe(true);

    await act(async () => {
      await result.current.unfreeze();
    });
    expect(result.current.isFrozen).toBe(false);
  });
});