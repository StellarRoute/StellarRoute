export type SubscriptionCadence = 'monthly' | 'weekly';

export interface SubscriptionCopyParams {
  amount: string;
  asset: string;
  cadence: SubscriptionCadence;
  recipient: string;
  maxCount: number;
}

/**
 * Generates user-facing copy for a subscription intent preview.
 *
 * The output reads as an amount, a cadence, a payee, and a maximum count,
 * with a note that each payment needs confirmation.
 *
 * Example: "15 USDC monthly to GABC for up to 12 payments. Each payment needs your confirmation."
 */
export function getSubscriptionCopy(params: SubscriptionCopyParams): string {
  const { amount, asset, cadence, recipient, maxCount } = params;
  return `${amount} ${asset} ${cadence} to ${recipient} for up to ${maxCount} payments. Each payment needs your confirmation.`;
}
