import { getWindowRecord, hasCallable, readPath } from '../detect';

export type Eip1193RequestArguments = {
  method: string;
  params?: unknown[] | Record<string, unknown>;
};

export type Eip1193Provider = {
  request: (args: Eip1193RequestArguments) => Promise<unknown>;
  on?: (event: string, listener: (...args: unknown[]) => void) => void;
  removeListener?: (
    event: string,
    listener: (...args: unknown[]) => void
  ) => void;
  isMetaMask?: boolean;
  providers?: Eip1193Provider[];
};

function asProvider(value: unknown): Eip1193Provider | null {
  if (!hasCallable(value, 'request')) return null;
  return value as Eip1193Provider;
}

/**
 * Resolve an injected EIP-1193 provider.
 * Prefers MetaMask when multiple providers are present.
 */
export function getInjectedEip1193Provider(): Eip1193Provider | null {
  const win = getWindowRecord();
  if (!win) return null;

  const ethereum = asProvider(win.ethereum);
  if (!ethereum) {
    // Some wallets expose only under their own namespace.
    const metamask = asProvider(readPath(win, ['ethereum']));
    return metamask;
  }

  if (Array.isArray(ethereum.providers) && ethereum.providers.length > 0) {
    const metamask = ethereum.providers.find((p) => p?.isMetaMask);
    return metamask ?? ethereum.providers[0] ?? ethereum;
  }

  return ethereum;
}

export async function eip1193Request<T>(
  provider: Eip1193Provider,
  method: string,
  params?: unknown[]
): Promise<T> {
  return (await provider.request({
    method,
    params,
  })) as T;
}
