function isBrowserLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof localStorage !== "undefined";
}

function removeCorruptKey(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore storage access errors in restricted browsing contexts
  }
}

/**
 * Reads and validates a JSON value from localStorage.
 * Returns `fallback` (and removes `key` when present) if parsing or shape validation fails.
 */
export function readJson<T, F = T>(
  key: string,
  guard: (v: unknown) => v is T,
  fallback: F,
): T | F {
  if (!isBrowserLocalStorage()) return fallback;

  let raw: string | null = null;
  try {
    raw = localStorage.getItem(key);
  } catch {
    return fallback;
  }

  if (raw === null) return fallback;

  try {
    const parsed: unknown = JSON.parse(raw);
    if (guard(parsed)) {
      return parsed;
    }
  } catch {
    // fall through to remove corrupt entry
  }

  removeCorruptKey(key);
  return fallback;
}
