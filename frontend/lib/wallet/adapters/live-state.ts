/**
 * Mutable live signing snapshot for sync getExecutionSupport().
 * Updated by connect / disconnect / network reads — never assumes always-true.
 */

export type LiveSigningSnapshot = {
  connected: boolean;
  networkMatch: boolean;
  canSign: boolean;
};

export function createLiveSigningTracker(
  initial: LiveSigningSnapshot = {
    connected: false,
    networkMatch: true,
    canSign: false,
  }
) {
  let snapshot: LiveSigningSnapshot = { ...initial };

  return {
    read(): LiveSigningSnapshot {
      return { ...snapshot };
    },
    patch(next: Partial<LiveSigningSnapshot>): void {
      snapshot = { ...snapshot, ...next };
    },
    reset(): void {
      snapshot = { ...initial };
    },
  };
}
