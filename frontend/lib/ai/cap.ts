/**
 * Per-confirm USDC spending cap for /ai (AI-31).
 *
 * A local guard that stops a mistyped amount before Confirm. It is stored only
 * in this browser and is not a custody control. Only /ai reads it — /swap does
 * not import this module.
 */

const CAP_STORAGE_KEY = 'stellar_route_ai_usdc_cap';

/** Parses a positive decimal cap; anything else means "no cap". */
function parseCap(raw: string | null | undefined): number | null {
  if (raw == null) return null;
  const trimmed = raw.trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return value > 0 ? value : null;
}

export function loadCap(): number | null {
  if (typeof window === 'undefined') return null;
  try {
    return parseCap(localStorage.getItem(CAP_STORAGE_KEY));
  } catch {
    return null;
  }
}

/** Saves a cap. An empty or invalid value clears it. Returns the stored cap. */
export function saveCap(raw: string): number | null {
  const cap = parseCap(raw);
  if (typeof window === 'undefined') return cap;
  try {
    if (cap === null) {
      localStorage.removeItem(CAP_STORAGE_KEY);
    } else {
      localStorage.setItem(CAP_STORAGE_KEY, String(cap));
    }
  } catch {
    // Storage unavailable: the cap still applies for this session via the return value.
  }
  return cap;
}

export function clearCap(): void {
  saveCap('');
}

/**
 * USDC-equivalent of an intent amount. USDC ("USDC" or "USDC:ISSUER") counts
 * one-to-one; other assets need a USDC rate. Returns null when unknown.
 */
export function usdcEquivalent(
  amount: string | undefined,
  asset: string | undefined,
  usdcRates: Record<string, number> = {},
): number | null {
  if (!amount || !asset) return null;
  const value = Number(amount);
  if (!Number.isFinite(value) || value < 0) return null;
  const code = asset.split(':')[0].trim().toUpperCase();
  if (code === 'USDC') return value;
  const rate = usdcRates[code];
  return typeof rate === 'number' && Number.isFinite(rate) ? value * rate : null;
}

/** True only when a cap is set and the known USDC-equivalent is above it. */
export function exceedsCap(
  amount: string | undefined,
  asset: string | undefined,
  cap: number | null,
  usdcRates?: Record<string, number>,
): boolean {
  if (cap === null) return false;
  const equivalent = usdcEquivalent(amount, asset, usdcRates);
  return equivalent !== null && equivalent > cap;
}

export function capMessage(cap: number): string {
  return `Above your ${cap} USDC per-confirm cap. Lower the amount or raise the cap in settings.`;
}
