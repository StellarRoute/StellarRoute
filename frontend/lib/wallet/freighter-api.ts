/**
 * Resolve Freighter API methods across CJS/UMD/ESM interop shapes.
 *
 * `@stellar/freighter-api` ships a webpack UMD bundle. Under Next/Turbopack
 * native ESM, named imports like `requestAccess` are often `undefined`, while
 * the real functions live on `default` / `module.exports`. Calling those
 * undefined named imports produces `(void 0) is not a function`.
 */

type FreighterApiError = { message?: string; code?: number };

type FreighterFnResult<T> = T & { error?: FreighterApiError };

export type FreighterApi = {
  isConnected: () => Promise<FreighterFnResult<{ isConnected: boolean }>>;
  requestAccess: () => Promise<FreighterFnResult<{ address: string }>>;
  getAddress: () => Promise<FreighterFnResult<{ address: string }>>;
  getNetworkDetails: () => Promise<
    FreighterFnResult<{
      network: string;
      networkUrl?: string;
      networkPassphrase?: string;
      sorobanRpcUrl?: string;
    }>
  >;
  signTransaction: (
    xdr: string,
    opts?: { networkPassphrase?: string; address?: string; network?: string },
  ) => Promise<
    FreighterFnResult<{ signedTxXdr: string; signerAddress?: string }>
  >;
};

function isFreighterApi(value: unknown): value is FreighterApi {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.isConnected === 'function' &&
    typeof candidate.requestAccess === 'function' &&
    typeof candidate.getAddress === 'function' &&
    typeof candidate.getNetworkDetails === 'function' &&
    typeof candidate.signTransaction === 'function'
  );
}

function unwrapCandidate(value: unknown): unknown {
  if (!value || typeof value !== 'object') return value;
  const record = value as Record<string, unknown>;
  if (isFreighterApi(record)) return record;
  if (isFreighterApi(record.default)) return record.default;
  if (isFreighterApi(record.freighterApi)) return record.freighterApi;
  if (isFreighterApi(record['module.exports'])) return record['module.exports'];
  return value;
}

export function resolveFreighterApi(moduleNamespace: unknown): FreighterApi {
  const candidates: unknown[] = [moduleNamespace, unwrapCandidate(moduleNamespace)];

  if (typeof window !== 'undefined') {
    const win = window as unknown as Record<string, unknown>;
    candidates.push(win.freighterApi, unwrapCandidate(win.freighterApi));
  }

  for (const candidate of candidates) {
    const resolved = unwrapCandidate(candidate);
    if (isFreighterApi(resolved)) return resolved;
  }

  throw new Error(
    'Freighter API failed to load in this browser bundle. Refresh the page, or install/enable the Freighter extension and try again.',
  );
}
