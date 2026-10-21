'use client';

import type { useCardFreezeAction } from '@/hooks/useCardFreezeAction';

type FreezeApi = ReturnType<typeof useCardFreezeAction>;

export interface CardFreezeButtonProps {
  api: FreezeApi;
}

export function CardFreezeButton({ api }: CardFreezeButtonProps) {
  const onClick = () => {
    if (api.isBusy) return;
    if (api.isFrozen) {
      void api.unfreeze();
    } else {
      void api.freeze();
    }
  };

  return (
    <div className="card-freeze-button-wrap">
      <button
        type="button"
        data-testid="card-freeze-button"
        data-lifecycle={api.lifecycle}
        disabled={api.isBusy}
        onClick={onClick}
        className="rounded-lg px-4 py-2 font-medium disabled:opacity-60"
      >
        {api.buttonLabel}
      </button>
      {api.lastTxHash && (
        <p data-testid="freeze-tx-hash" className="mt-2 text-sm text-slate-500">
          Tx: {api.lastTxHash}
        </p>
      )}
      {api.error && (
        <p data-testid="freeze-error" className="mt-2 text-sm text-red-600">
          {api.error}
        </p>
      )}
    </div>
  );
}