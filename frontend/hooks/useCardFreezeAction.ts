'use client';

import { useCallback, useState } from 'react';

export type FreezeLifecycle =
  | 'idle'
  | 'awaiting_signature'
  | 'submitting'
  | 'completed';

export type FreezeAction = 'freeze' | 'unfreeze';

export interface FreezeResult {
  ok: boolean;
  txHash?: string;
  error?: string;
  rejected?: boolean;
}

export interface UseCardFreezeActionOptions {
  /**
   * Optional injector for tests / real Freighter later.
   * Default simulates a successful sign + broadcast.
   */
  requestSignature?: (action: FreezeAction) => Promise<FreezeResult>;
  onSuccess?: (action: FreezeAction, txHash: string) => void;
  onRejected?: () => void;
}

const defaultRequestSignature = async (
  action: FreezeAction,
): Promise<FreezeResult> => {
  // Simulated wallet + network delay (replace with Freighter in production path)
  await new Promise((r) => setTimeout(r, 50));
  return {
    ok: true,
    txHash: `SIM-${action.toUpperCase()}-${Date.now()}`,
  };
};

export function useCardFreezeAction(options: UseCardFreezeActionOptions = {}) {
  const requestSignature = options.requestSignature ?? defaultRequestSignature;

  const [lifecycle, setLifecycle] = useState<FreezeLifecycle>('idle');
  const [isFrozen, setIsFrozen] = useState(false);
  const [lastTxHash, setLastTxHash] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(
    async (action: FreezeAction) => {
      setError(null);
      setLifecycle('awaiting_signature');

      try {
        const result = await requestSignature(action);

        if (result.rejected) {
          setLifecycle('idle');
          options.onRejected?.();
          return result;
        }

        if (!result.ok) {
          setLifecycle('idle');
          setError(result.error ?? 'Transaction failed');
          return result;
        }

        setLifecycle('submitting');
        // brief submitting phase
        await new Promise((r) => setTimeout(r, 30));

        const txHash = result.txHash ?? 'UNKNOWN';
        setLastTxHash(txHash);
        setIsFrozen(action === 'freeze');
        setLifecycle('completed');
        options.onSuccess?.(action, txHash);

        // return to idle so button can be used again
        setLifecycle('idle');
        return result;
      } catch {
        setLifecycle('idle');
        setError('Unexpected error');
        return { ok: false, error: 'Unexpected error' } satisfies FreezeResult;
      }
    },
    [requestSignature, options],
  );

  const freeze = useCallback(() => run('freeze'), [run]);
  const unfreeze = useCallback(() => run('unfreeze'), [run]);

  const isBusy =
    lifecycle === 'awaiting_signature' || lifecycle === 'submitting';

  const buttonLabel =
    lifecycle === 'awaiting_signature'
      ? 'Awaiting Signature...'
      : lifecycle === 'submitting'
        ? 'Broadcasting...'
        : isFrozen
          ? 'Unfreeze Card'
          : 'Freeze Card';

  return {
    lifecycle,
    isFrozen,
    isBusy,
    buttonLabel,
    lastTxHash,
    error,
    freeze,
    unfreeze,
    setIsFrozen, // allow parent hydrate
  };
}