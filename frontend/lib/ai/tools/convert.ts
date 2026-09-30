/**
 * AI-14 Convert preview (closes #1424).
 *
 * Additive-only: new file, no edits to quote ranking, swap prepare, or the
 * existing quote client. Calls the existing quote client read-only (GET) and
 * renders amount out + venue + degraded notice. Never POSTs to swap prepare.
 * Confirm is a deep-link to /swap with prefilled params (wallet signs there).
 */

import type { PriceQuote } from '../../../types/index';

/** Confirmed-preview convert intent (already parsed + user-confirmed preview). */
export interface ConvertPreviewInput {
  fromAsset: string;
  toAsset: string;
  /** Decimal amount string, e.g. "10" */
  amount: string;
}

/** Minimal read surface of the existing quote client (injectable for tests). */
export interface ConvertQuoteReader {
  getQuote(
    base: string,
    quote: string,
    amount?: number,
    type?: 'sell' | 'buy',
  ): Promise<{ quote: PriceQuote }>;
}

export interface ConvertPreviewResult {
  kind: 'preview';
  summary: string;
  amountOut: string;
  venue: string;
  degraded: boolean;
  notice: string | null;
  /** Deep-link handoff; the wallet signs on /swap, never here. */
  swapHandoffUrl: string;
}

const SWAP_PREPARE_GUARD = '/api/v1/swap/prepare';

function assertNoSwapPrepare(url: string): void {
  if (url.includes(SWAP_PREPARE_GUARD)) {
    throw new Error('convert preview must never call swap prepare');
  }
}

function buildSwapHandoffUrl(input: ConvertPreviewInput): string {
  const params = new URLSearchParams({
    from: input.fromAsset,
    to: input.toAsset,
    amount: input.amount,
  });
  const url = `/swap?${params.toString()}`;
  assertNoSwapPrepare(url);
  return url;
}

/**
 * Render a confirmed-preview convert intent using the existing quote client.
 * Read-only: exactly one GET quote call, zero swap-prepare POSTs.
 */
export async function previewConvert(
  input: ConvertPreviewInput,
  reader: ConvertQuoteReader,
): Promise<ConvertPreviewResult> {
  const amount = Number(input.amount);
  if (!input.fromAsset || !input.toAsset || !Number.isFinite(amount) || amount <= 0) {
    throw new Error('fromAsset, toAsset and a positive amount are required');
  }

  const { quote } = await reader.getQuote(input.fromAsset, input.toAsset, amount, 'sell');

  const degraded = quote.degraded === true;
  const venue = quote.path?.[0]?.source ?? 'sdex';
  const notice = degraded
    ? 'Quote is serving degraded market data — output shown as-is for review.'
    : null;

  return {
    kind: 'preview',
    summary: `Convert ${input.amount} ${input.fromAsset} → ${input.toAsset}`,
    amountOut: quote.total,
    venue,
    degraded,
    notice,
    swapHandoffUrl: buildSwapHandoffUrl(input),
  };
}
