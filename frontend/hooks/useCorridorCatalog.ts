import { useMemo, useSyncExternalStore } from 'react';
import type { CorridorDefinition, CorridorId } from '@/lib/cross-chain/types';
import {
  CORRIDOR_CATALOG,
  findCorridorById,
  isCorridorExecutable,
  resolveCorridorAvailability,
} from '@/lib/cross-chain/corridors';
import {
  getReadinessSnapshot,
  subscribeReadiness,
} from '@/lib/cctp/readiness';

export function useCorridorCatalog() {
  // Recompute when /api/v2 readiness updates CCTP route registration.
  const readiness = useSyncExternalStore(
    subscribeReadiness,
    getReadinessSnapshot,
    getReadinessSnapshot
  );

  const corridors = useMemo(
    () =>
      CORRIDOR_CATALOG.map((corridor) => ({
        ...corridor,
        availability: resolveCorridorAvailability(corridor),
        executable: isCorridorExecutable(corridor),
      })),
    [readiness.fetchedAt, readiness.corridors]
  );

  const executableCorridors = useMemo(
    () => corridors.filter((c) => c.executable),
    [corridors]
  );

  return {
    corridors,
    executableCorridors,
    getCorridor: (id: CorridorId) => {
      const base = findCorridorById(id);
      return {
        ...base,
        availability: resolveCorridorAvailability(base),
        executable: isCorridorExecutable(base),
      };
    },
  };
}

export type EnrichedCorridor = CorridorDefinition & {
  availability: 'executable' | 'coming_soon';
  executable: boolean;
};
