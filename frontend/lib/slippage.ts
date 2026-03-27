export const SLIPPAGE_PRESETS = [0.1, 0.5, 1];

export const MIN_SLIPPAGE = 0;
export const MAX_SLIPPAGE = 50;

export const LOW_SLIPPAGE_WARNING_THRESHOLD = 0.1;
export const HIGH_SLIPPAGE_WARNING_THRESHOLD = 1;

export function parseSlippageInput(value: string): number | null {
  if (value.trim() === "") return null;

  const parsed = Number(value);

  if (Number.isNaN(parsed)) return null;

  return parsed;
}

export function isValidSlippage(value: number): boolean {
  return value >= MIN_SLIPPAGE && value <= MAX_SLIPPAGE;
}

export function getSlippageWarning(value: number | null): string | null {
  if (value === null) return null;

  if (value < LOW_SLIPPAGE_WARNING_THRESHOLD) {
    return "Very low slippage may cause the swap to fail.";
  }

  if (value > HIGH_SLIPPAGE_WARNING_THRESHOLD) {
    return "High slippage increases the risk of receiving a worse price.";
  }

  return null;
}

export function getSlippageWarningLevel(
  value: number | null,
): "low" | "high" | null {
  if (value === null) return null;

  if (value < LOW_SLIPPAGE_WARNING_THRESHOLD) {
    return "low";
  }

  if (value > HIGH_SLIPPAGE_WARNING_THRESHOLD) {
    return "high";
  }

  return null;
}
