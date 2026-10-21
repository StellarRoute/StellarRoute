/**
 * Cash-out (Offramp) Quote Builder
 *
 * Builds an indicative NGN cash-out preview for the AI agent by calling
 * the existing offramp quote helper. It does not collect a new bank form,
 * read any Paycrest secret, or invent a Stellar deposit network.
 *
 * Non-USDC sources keep the existing status from offramp types
 * (bridge_required / swap_then_offramp) as copy, not a new executor.
 *
 * This module is additive-only: it does not change any existing swap,
 * quote, or OpenAPI contract behavior.
 */

import {
  buildOfframpQuotePreview,
  findOfframpSource,
  resolveOfframpMode,
  OFFRAMP_SOURCE_ASSETS,
  type OfframpQuotePreview,
  type OfframpSourceAsset,
} from '@/lib/offramp';

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export type OfframpToolErrorCode =
  | 'invalid_asset'
  | 'invalid_amount'
  | 'source_not_found'
  | 'quote_failed';

export class OfframpToolError extends Error {
  constructor(
    public readonly code: OfframpToolErrorCode,
    message: string,
  ) {
    super(message);
    this.name = 'OfframpToolError';
  }
}

// ---------------------------------------------------------------------------
// Asset resolution
// ---------------------------------------------------------------------------

/**
 * Resolve an AI-intent asset identifier to an OfframpSourceAsset.
 *
 * Accepts:
 *   - Offramp source IDs  (e.g. "stellar-usdc", "stellar-xlm", "eth-usdc")
 *   - Common symbols       (e.g. "USDC", "XLM")
 *
 * Throws OfframpToolError with code 'invalid_asset' when the identifier
 * cannot be matched to a known offramp source.
 */
export function resolveOfframpAsset(
  asset: string,
): OfframpSourceAsset {
  const trimmed = asset.trim();
  if (!trimmed) {
    throw new OfframpToolError('invalid_asset', 'Asset identifier is required');
  }

  const lower = trimmed.toLowerCase();

  // Try exact source ID match first (e.g. "stellar-usdc", "stellar-xlm")
  const byId = findOfframpSource(lower);
  if (byId) return byId;

  // Try matching by symbol field (e.g. "USDC", "XLM")
  const bySymbol = OFFRAMP_SOURCE_ASSETS.find(
    (a) => a.symbol.toLowerCase() === lower,
  );
  if (bySymbol) return bySymbol;

  throw new OfframpToolError(
    'invalid_asset',
    `Unknown offramp asset: "${asset}"`,
  );
}

// ---------------------------------------------------------------------------
// Quote builder
// ---------------------------------------------------------------------------

export interface BuildCashOutQuoteParams {
  /** Asset identifier: source ID (e.g. "stellar-usdc") or symbol (e.g. "USDC") */
  asset: string;
  /** Positive decimal amount string */
  amount: string;
  /** Optional override mode; defaults to the asset's resolved mode */
  mode?: 'direct' | 'bridge';
}

/**
 * Build an indicative cash-out quote preview for the AI agent.
 *
 * Calls the existing offramp quote helper and returns NGN, fee, and the
 * indicative flag. Non-USDC sources keep the existing status from offramp
 * types (bridge_required / swap_then_offramp) as copy, not a new executor.
 *
 * Returns null when the amount is invalid or the source cannot be resolved,
 * matching the behavior of the existing `buildOfframpQuotePreview` helper.
 */
export async function buildCashOutQuote(
  params: BuildCashOutQuoteParams,
): Promise<OfframpQuotePreview | null> {
  const amount = params.amount.trim();
  if (!amount || isNaN(Number(amount)) || Number(amount) <= 0) {
    return null;
  }

  const asset = resolveOfframpAsset(params.asset);
  const mode = params.mode ?? resolveOfframpMode(asset);

  return buildOfframpQuotePreview({
    asset,
    amount,
    mode,
  });
}
