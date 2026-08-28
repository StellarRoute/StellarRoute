import {
  eip1193Request,
  getInjectedEip1193Provider,
  type Eip1193Provider,
} from '@/lib/wallet/adapters/evm/provider';

export const DEFAULT_RECEIPT_POLL_MS = 2_000;
export const DEFAULT_RECEIPT_TIMEOUT_MS = 120_000;

export type EvmReceiptStatus = 'success' | 'reverted' | 'pending';

export type EvmReceiptPollDeps = {
  getProvider?: () => Eip1193Provider | null;
  pollReceipt?: (
    provider: Eip1193Provider,
    txHash: string,
    opts: { signal?: AbortSignal; timeoutMs?: number; pollMs?: number },
  ) => Promise<EvmReceiptStatus>;
};

export async function pollEvmTransactionReceipt(
  txHash: string,
  opts: {
    signal?: AbortSignal;
    timeoutMs?: number;
    pollMs?: number;
    deps?: EvmReceiptPollDeps;
  } = {},
): Promise<EvmReceiptStatus> {
  const provider = opts.deps?.getProvider?.() ?? getInjectedEip1193Provider();
  if (!provider) return 'pending';
  const poll =
    opts.deps?.pollReceipt ??
    ((p, hash, inner) => pollReceiptViaProvider(p, hash, inner));
  return poll(provider, txHash, {
    signal: opts.signal,
    timeoutMs: opts.timeoutMs ?? DEFAULT_RECEIPT_TIMEOUT_MS,
    pollMs: opts.pollMs ?? DEFAULT_RECEIPT_POLL_MS,
  });
}

export async function pollReceiptViaProvider(
  provider: Eip1193Provider,
  txHash: string,
  opts: { signal?: AbortSignal; timeoutMs?: number; pollMs?: number } = {},
): Promise<EvmReceiptStatus> {
  const timeoutMs = opts.timeoutMs ?? DEFAULT_RECEIPT_TIMEOUT_MS;
  const pollMs = opts.pollMs ?? DEFAULT_RECEIPT_POLL_MS;
  const started = Date.now();

  while (Date.now() - started < timeoutMs) {
    if (opts.signal?.aborted) return 'pending';
    try {
      const receipt = await eip1193Request<{ status?: string } | null>(
        provider,
        'eth_getTransactionReceipt',
        [txHash],
      );
      if (receipt) {
        const status = receipt.status ?? '0x1';
        return status === '0x0' ? 'reverted' : 'success';
      }
    } catch {
      // keep polling until timeout — wallet/provider may be slow
    }
    await sleep(pollMs, opts.signal);
  }
  return 'pending';
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal?.aborted) {
      resolve();
      return;
    }
    const timer = setTimeout(resolve, ms);
    signal?.addEventListener(
      'abort',
      () => {
        clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
  });
}
