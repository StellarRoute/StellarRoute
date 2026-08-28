import { describe, expect, it } from 'vitest';
import {
  CORRIDOR_CATALOG,
  catalogMatchesBackendRoutes,
  chainFamilyForDisplayId,
  isCorridorExecutable,
  resolveCorridorAvailability,
} from './corridors';
import { hasBackendRoute } from '@/lib/wallet/adapters';

describe('corridor catalog', () => {
  it('keeps catalog availability synchronized with hasBackendRoute', () => {
    expect(catalogMatchesBackendRoutes()).toBe(true);
  });

  it('marks stellar-native and both Sepolia↔Stellar CCTP corridors as executable', () => {
    const executable = CORRIDOR_CATALOG.filter((c) => isCorridorExecutable(c)).map(
      (c) => c.id
    );
    expect(executable).toEqual([
      'stellar-native',
      'evm-to-stellar',
      'stellar-to-evm',
    ]);
  });

  it('reflects backend route registration per corridor leg', () => {
    for (const corridor of CORRIDOR_CATALOG) {
      const source = chainFamilyForDisplayId(corridor.sourceChainId);
      const dest = chainFamilyForDisplayId(corridor.destChainId);
      const backend = hasBackendRoute(source, dest);
      const availability = resolveCorridorAvailability(corridor);
      if (backend) {
        expect(availability).toBe('executable');
      } else {
        expect(availability).toBe('coming_soon');
      }
    }
  });
});
