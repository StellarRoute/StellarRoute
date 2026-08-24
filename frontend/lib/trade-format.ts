// frontend/lib/trade-format.ts

export function truncateTxHash(hash: string): string {
  if (hash.length <= 12) return hash;
  return hash.slice(0, 8) + "…" + hash.slice(-4);
}

export function formatTradeTimestamp(date: Date): string {
  const year = Math.abs(date.getUTCFullYear()) % 10000;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${year.toString().padStart(4, '0')}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())} UTC`;
}

export function formatTradeAmount(amount: string): string {
  const num = parseFloat(amount);
  if (isNaN(num)) return amount;
  
  // Use Number() to convert to a clean number, then string
  // This effectively handles trailing zeros automatically (e.g., 1.50 -> 1.5)
  // We only return "0" if the number is actually 0.
  return Number(num.toFixed(7)).toString();
}

export function stellarExplorerUrl(txHash: string): string {
  return `https://stellar.expert/explorer/public/tx/${txHash}`;
}