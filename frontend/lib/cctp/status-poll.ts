import type { CctpTransferStatus, CctpTransferStatusResponse } from './types';
import type { CctpApiClient } from './client';

const TERMINAL: ReadonlySet<CctpTransferStatus> = new Set([
  'completed',
  'cancelled',
  'provider_killed',
]);

const POLLABLE: ReadonlySet<CctpTransferStatus> = new Set([
  'burn_submitted',
  'awaiting_attestation',
  'attestation_ready',
  'mint_prepared',
  'mint_submitted',
  'attestation_failed',
  'mint_failed_retryable',
]);

export type StatusPollCallbacks = {
  onUpdate: (status: CctpTransferStatusResponse) => void;
  onTerminal?: (status: CctpTransferStatusResponse) => void;
  onError?: (err: unknown) => void;
};

export type StatusPollHandle = {
  stop: () => void;
};

export function startCctpStatusPoll(input: {
  client: CctpApiClient;
  transferId: string;
  accessToken: string;
  callbacks: StatusPollCallbacks;
  maxMs?: number;
}): StatusPollHandle {
  const controller = new AbortController();
  let attempt = 0;
  let stopped = false;
  const started = Date.now();
  const maxMs = input.maxMs ?? 15 * 60 * 1000;

  const tick = async () => {
    if (stopped || controller.signal.aborted) return;
    if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
      schedule();
      return;
    }
    if (typeof navigator !== 'undefined' && !navigator.onLine) {
      schedule();
      return;
    }
    if (Date.now() - started > maxMs) {
      input.callbacks.onError?.(new Error('Status polling timed out'));
      return;
    }

    try {
      const status = await input.client.getTransfer(input.transferId, {
        accessToken: input.accessToken,
        signal: controller.signal,
      });
      input.callbacks.onUpdate(status);
      if (TERMINAL.has(status.status)) {
        input.callbacks.onTerminal?.(status);
        return;
      }
      if (POLLABLE.has(status.status)) {
        schedule();
      }
    } catch (err) {
      if (controller.signal.aborted) return;
      input.callbacks.onError?.(err);
      schedule();
    }
  };

  const schedule = () => {
    if (stopped) return;
    const delay = Math.min(30_000, 1_500 * Math.pow(1.6, attempt)) + Math.random() * 400;
    attempt += 1;
    setTimeout(() => void tick(), delay);
  };

  void tick();

  return {
    stop: () => {
      stopped = true;
      controller.abort();
    },
  };
}
