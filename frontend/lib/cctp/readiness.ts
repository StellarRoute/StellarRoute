import type { ChainFamily } from '@/lib/wallet/adapters';
import { routeKey, setCctpExecutableRoutes } from '@/lib/wallet/adapters/execution-support';
import type { SupportedCorridor } from './types';
import { getCctpApiClient } from './client';

export type ReadinessSnapshot = {
  loaded: boolean;
  corridors: SupportedCorridor[];
  cctpGloballyReady: boolean;
  providerKilled: boolean;
  error: string | null;
  fetchedAt: number | null;
};

let snapshot: ReadinessSnapshot = {
  loaded: false,
  corridors: [],
  cctpGloballyReady: false,
  providerKilled: false,
  error: null,
  fetchedAt: null,
};

const listeners = new Set<() => void>();

function chainFamilyFromCaip(chainId: string): ChainFamily | null {
  if (chainId.startsWith('stellar:')) return 'stellar';
  if (chainId.startsWith('eip155:')) return 'evm';
  return null;
}

export function applyReadinessCorridors(corridors: SupportedCorridor[]): void {
  const executable = corridors
    .filter((c) => c.executable)
    .map((c) => {
      const source = chainFamilyFromCaip(c.source_chain_id);
      const dest = chainFamilyFromCaip(c.destination_chain_id);
      if (!source || !dest) return null;
      return { source, destination: dest };
    })
    .filter(Boolean) as Array<{ source: ChainFamily; destination: ChainFamily }>;

  setCctpExecutableRoutes(executable);
  snapshot = {
    ...snapshot,
    loaded: true,
    corridors,
    cctpGloballyReady: corridors.some((c) => c.executable),
    providerKilled: corridors.length > 0 && corridors.every((c) => !c.executable),
    fetchedAt: Date.now(),
  };
  emit();
}

export function getReadinessSnapshot(): ReadinessSnapshot {
  return snapshot;
}

export function subscribeReadiness(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function emit() {
  for (const l of listeners) l();
}

export async function refreshApiV2Readiness(signal?: AbortSignal): Promise<ReadinessSnapshot> {
  try {
    const info = await getCctpApiClient().getApiV2Info(signal);
    const corridors = info.supported_corridors ?? [];
    applyReadinessCorridors(corridors);
    const anyExecutable = corridors.some((c) => c.executable);
    snapshot = {
      loaded: true,
      corridors,
      cctpGloballyReady: anyExecutable || info.bridge_settlement_executable,
      providerKilled: corridors.length > 0 && corridors.every((c) => !c.executable),
      error: null,
      fetchedAt: Date.now(),
    };
  } catch (err) {
    setCctpExecutableRoutes([]);
    snapshot = {
      ...snapshot,
      loaded: true,
      corridors: [],
      cctpGloballyReady: false,
      providerKilled: false,
      error: err instanceof Error ? err.message : 'Failed to load bridge readiness',
      fetchedAt: Date.now(),
    };
  }
  emit();
  return snapshot;
}

export function findExecutableCorridor(
  sourceChainId: string,
  destChainId: string,
): SupportedCorridor | undefined {
  return snapshot.corridors.find(
    (c) =>
      c.executable &&
      c.source_chain_id === sourceChainId &&
      c.destination_chain_id === destChainId,
  );
}

export function isRouteExecutable(source: ChainFamily, dest: ChainFamily): boolean {
  const key = routeKey(source, dest);
  return snapshot.corridors.some((c) => {
    const s = chainFamilyFromCaip(c.source_chain_id);
    const d = chainFamilyFromCaip(c.destination_chain_id);
    return c.executable && s && d && routeKey(s, d) === key;
  });
}

export function resetReadinessForTests(): void {
  snapshot = {
    loaded: false,
    corridors: [],
    cctpGloballyReady: false,
    providerKilled: false,
    error: null,
    fetchedAt: null,
  };
  setCctpExecutableRoutes([]);
  emit();
}
