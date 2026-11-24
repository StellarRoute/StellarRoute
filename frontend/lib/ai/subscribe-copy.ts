/**
 * Subscription schedule copy and intent preview generation.
 * Formats repeated payment intents into clear, human-readable previews
 * ensuring that users understand payment amounts, cadences, payees, max counts,
 * and that each payment requires explicit confirmation (no automated timer pull).
 */

export type SubscriptionCadence = 'monthly' | 'weekly';

export interface SubscriptionScheduleOptions {
  /**
   * Amount per individual payment cycle (e.g. "15", 15).
   */
  amount: string | number;

  /**
   * Currency or asset symbol (e.g. "USDC", "XLM").
   */
  asset: string;

  /**
   * Recurring cadence: 'monthly' | 'weekly'.
   */
  cadence: SubscriptionCadence;

  /**
   * Recipient address or destination account (e.g. Stellar G-address).
   */
  payee: string;

  /**
   * Maximum total count of recurring payments (optional).
   */
  maxCount?: number;

  /**
   * Optional payment memo or reference note.
   */
  memo?: string;
}

export interface SubscriptionScheduleCopy {
  /**
   * Concise one-line preview headline.
   */
  headline: string;

  /**
   * Capitalized cadence label (e.g. "Monthly", "Weekly").
   */
  cadenceLabel: string;

  /**
   * Formatted amount with asset symbol (e.g. "15 USDC").
   */
  amountFormatted: string;

  /**
   * Full payee address.
   */
  payee: string;

  /**
   * Abbreviated payee address (e.g. "GABC...WXYZ").
   */
  payeeShort: string;

  /**
   * Total payment count label (e.g. "Up to 12 payments" or "1 payment").
   */
  maxCountLabel: string;

  /**
   * Explicit confirmation safety notice stating that funds are never pulled on a timer
   * and that each payment requires user confirmation.
   */
  confirmationNotice: string;

  /**
   * Total maximum expenditure across all cycles (e.g. "180 USDC"), or null if unbounded.
   */
  totalMaxFormatted: string | null;

  /**
   * Complete descriptive sentence for natural language agent responses.
   */
  fullDescription: string;
}

/**
 * Shortens a public address to G... format for readable inline copy.
 */
export function truncatePayee(
  address: string,
  frontChars = 4,
  backChars = 4
): string {
  if (!address || typeof address !== 'string') {
    return '';
  }
  const clean = address.trim();
  if (clean.length <= frontChars + backChars + 3) {
    return clean;
  }
  return `${clean.slice(0, frontChars)}…${clean.slice(-backChars)}`;
}

/**
 * Standard confirmation safety notice for subscription intents.
 */
export const CONFIRMATION_SAFETY_NOTICE =
  'Each payment requires manual confirmation. Funds will not be pulled automatically on a timer.';

/**
 * Generates structured copy and human-readable preview for a subscription schedule.
 */
export function formatSubscriptionSchedule(
  options: SubscriptionScheduleOptions
): SubscriptionScheduleCopy {
  const { amount, asset, cadence, payee, maxCount, memo } = options;

  const normalizedAsset = (asset || '').trim().toUpperCase();
  const numericAmount =
    typeof amount === 'string' ? parseFloat(amount) : amount;
  const validAmount =
    Number.isFinite(numericAmount) && numericAmount > 0 ? numericAmount : 0;
  const amountFormatted = `${validAmount} ${normalizedAsset}`;

  const cadenceLower = (
    cadence || 'monthly'
  ).toLowerCase() as SubscriptionCadence;
  const cadenceLabel = cadenceLower === 'weekly' ? 'Weekly' : 'Monthly';
  const cadenceAdverb = cadenceLower === 'weekly' ? 'weekly' : 'monthly';

  const cleanPayee = (payee || '').trim();
  const payeeShort = truncatePayee(cleanPayee);

  const hasMaxCount =
    typeof maxCount === 'number' && Number.isFinite(maxCount) && maxCount > 0;
  const cleanMaxCount = hasMaxCount ? Math.floor(maxCount) : undefined;

  let maxCountLabel: string;
  let totalMaxFormatted: string | null = null;

  if (cleanMaxCount !== undefined) {
    const cycleWord = cleanMaxCount === 1 ? 'payment' : 'payments';
    maxCountLabel = `Up to ${cleanMaxCount} ${cycleWord}`;
    const totalMax = (validAmount * cleanMaxCount)
      .toFixed(2)
      .replace(/\.00$/, '');
    totalMaxFormatted = `${totalMax} ${normalizedAsset} maximum`;
  } else {
    maxCountLabel = 'Recurring until canceled';
  }

  // Construct headline
  let headline = `Pay ${amountFormatted} ${cadenceAdverb} to ${payeeShort}`;
  if (cleanMaxCount !== undefined) {
    headline += ` (max ${cleanMaxCount} ${cleanMaxCount === 1 ? 'payment' : 'payments'})`;
  }

  // Construct full plain-English description
  let fullDescription = `Send ${amountFormatted} ${cadenceAdverb} to ${cleanPayee}`;
  if (cleanMaxCount !== undefined) {
    fullDescription += ` for a maximum of ${cleanMaxCount} ${cleanMaxCount === 1 ? 'payment' : 'payments'}`;
    if (totalMaxFormatted) {
      fullDescription += ` (up to ${totalMaxFormatted})`;
    }
  }
  if (memo && memo.trim()) {
    fullDescription += ` with memo "${memo.trim()}"`;
  }
  fullDescription += `. ${CONFIRMATION_SAFETY_NOTICE}`;

  return {
    headline,
    cadenceLabel,
    amountFormatted,
    payee: cleanPayee,
    payeeShort,
    maxCountLabel,
    confirmationNotice: CONFIRMATION_SAFETY_NOTICE,
    totalMaxFormatted,
    fullDescription,
  };
}

/**
 * Returns a concise plain-text preview string for subscription intent cards.
 */
export function renderSubscriptionPreview(
  options: SubscriptionScheduleOptions
): string {
  const schedule = formatSubscriptionSchedule(options);
  const lines = [
    `Schedule: ${schedule.headline}`,
    `Cadence: ${schedule.cadenceLabel}`,
    `Amount per payment: ${schedule.amountFormatted}`,
    `Payee: ${schedule.payee}`,
    `Duration: ${schedule.maxCountLabel}`,
  ];

  if (schedule.totalMaxFormatted) {
    lines.push(`Total ceiling: ${schedule.totalMaxFormatted}`);
  }

  lines.push(`Confirmation: ${schedule.confirmationNotice}`);

  return lines.join('\n');
}

/**
 * Parses conversational subscription prompts like:
 * "pay 15 USDC monthly to G... up to 12 payments"
 * "pay 10 XLM weekly to G... for 4 weeks"
 */
export function parseSubscriptionPrompt(
  prompt: string
): SubscriptionScheduleOptions | null {
  if (!prompt || typeof prompt !== 'string') {
    return null;
  }

  const clean = prompt.trim();

  // Pattern: pay <amount> <asset> <monthly|weekly> to <payee> [optional: max <N> / up to <N> / for <N>]
  const regex =
    /pay\s+(\d+(?:\.\d+)?)\s+([a-zA-Z0-9]+)\s+(monthly|weekly)\s+to\s+([a-zA-Z0-9_\-\.]+)(?:.*?(?:up\s+to|max|for)\s+(\d+))?/i;

  const match = clean.match(regex);
  if (!match) {
    return null;
  }

  const [, amountStr, asset, cadenceStr, payee, countStr] = match;

  const cadence = cadenceStr.toLowerCase() as SubscriptionCadence;
  const maxCount = countStr ? parseInt(countStr, 10) : undefined;

  return {
    amount: amountStr,
    asset: asset.toUpperCase(),
    cadence,
    payee,
    maxCount: maxCount && maxCount > 0 ? maxCount : undefined,
  };
}
