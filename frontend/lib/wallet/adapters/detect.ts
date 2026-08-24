/**
 * Safe browser-provider detection helpers.
 * Presence checks must not throw and should time out quickly.
 */

export function getWindowRecord(): Record<string, unknown> | null {
  return typeof window === 'undefined'
    ? null
    : (window as unknown as Record<string, unknown>);
}

function getTimerFns(): {
  setTimeout: (handler: () => void, ms: number) => number | ReturnType<typeof globalThis.setTimeout>;
  clearTimeout: (id: number | ReturnType<typeof globalThis.setTimeout>) => void;
} | null {
  const g = globalThis as typeof globalThis & {
    setTimeout?: typeof globalThis.setTimeout;
    clearTimeout?: typeof globalThis.clearTimeout;
  };
  if (typeof g.setTimeout !== 'function' || typeof g.clearTimeout !== 'function') {
    return null;
  }
  return {
    setTimeout: g.setTimeout.bind(g),
    clearTimeout: g.clearTimeout.bind(g),
  };
}

/**
 * Race a promise against a timeout. SSR-safe: uses `globalThis` timers,
 * never assumes `window` exists.
 */
export function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
  fallback: T
): Promise<T> {
  const timers = getTimerFns();
  if (!timers) {
    return promise.catch(() => fallback);
  }

  return new Promise((resolve) => {
    const timer = timers.setTimeout(() => resolve(fallback), ms);
    promise
      .then((value) => {
        timers.clearTimeout(timer);
        resolve(value);
      })
      .catch(() => {
        timers.clearTimeout(timer);
        resolve(fallback);
      });
  });
}

/** Nested property read without throwing (e.g. `okxwallet.bitcoin`). */
export function readPath(
  root: Record<string, unknown> | null,
  path: string[]
): unknown {
  let current: unknown = root;
  for (const key of path) {
    if (!current || typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

export function hasCallable(value: unknown, method: string): boolean {
  if (!value || typeof value !== 'object') return false;
  return typeof (value as Record<string, unknown>)[method] === 'function';
}
