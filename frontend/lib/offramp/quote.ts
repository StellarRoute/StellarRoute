import type {
  OfframpMode,
  OfframpQuotePreview,
  OfframpRouteStep,
  OfframpSourceAsset,
} from './types';

/**
 * Indicative USDC→NGN mid for UI preview only.
 * Live partner rates will replace this when payout rails go live.
 */
export const INDICATIVE_USDC_NGN = 1_580;

/** Flat preview fee in basis points (50 = 0.5%). */
export const OFFRAMP_FEE_BPS = 50;

const XLM_TO_USDC_INDICATIVE = 0.12;

function parsePositiveAmount(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed);
  if (!Number.isFinite(value) || value <= 0) return null;
  return value;
}

function formatMoney(value: number, decimals: number): string {
  return value.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: decimals,
  });
}

/** Convert source notional into approximate USDC for preview quotes. */
export function estimateUsdcFromSource(
  asset: OfframpSourceAsset,
  amount: number,
): number {
  if (asset.kind === 'stellar_xlm') {
    return amount * XLM_TO_USDC_INDICATIVE;
  }
  // USDC and other stables: 1:1 for preview (before fee).
  return amount;
}

export function buildOfframpQuotePreview(input: {
  asset: OfframpSourceAsset;
  amount: string;
  mode: OfframpMode;
}): OfframpQuotePreview | null {
  const amount = parsePositiveAmount(input.amount);
  if (amount == null) return null;

  const usdc = estimateUsdcFromSource(input.asset, amount);
  const fee = (usdc * OFFRAMP_FEE_BPS) / 10_000;
  const net = Math.max(usdc - fee, 0);
  const receiveNgn = net * INDICATIVE_USDC_NGN;

  const etaLabel =
    input.mode === 'direct'
      ? 'Usually under 15 minutes once payout is live'
      : 'Bridge finality + payout — typically 20–45 minutes';

  return {
    sourceAmount: formatMoney(amount, Math.min(input.asset.decimals, 6)),
    sourceSymbol: input.asset.symbol,
    usdcAmount: formatMoney(usdc, 6),
    feeUsdc: formatMoney(fee, 6),
    netUsdc: formatMoney(net, 6),
    rateNgn: INDICATIVE_USDC_NGN,
    receiveNgn: formatMoney(receiveNgn, 2),
    etaLabel,
    mode: input.mode,
    indicative: true,
  };
}

export function buildOfframpRouteSteps(
  asset: OfframpSourceAsset,
  mode: OfframpMode,
): OfframpRouteStep[] {
  const onStellar =
    asset.kind === 'stellar_usdc' || asset.kind === 'stellar_xlm';
  const needsBridge =
    mode === 'bridge' && !asset.isStellarUsdc && !onStellar;
  const needsSwap = asset.status === 'swap_then_offramp';

  const steps: OfframpRouteStep[] = [
    {
      id: 'source',
      label: `${asset.symbol} on ${asset.chainLabel}`,
      detail: needsSwap
        ? 'You send this asset; we route into Stellar USDC first.'
        : 'You send from the selected chain wallet.',
      active: true,
    },
  ];

  if (needsBridge) {
    steps.push({
      id: 'bridge',
      label: 'Bridge to Stellar',
      detail: 'Circle CCTP burns source USDC and mints native Stellar USDC.',
      active: asset.status !== 'coming_soon',
      optional: asset.status === 'coming_soon',
    });
  }

  if (needsSwap) {
    steps.push({
      id: 'settle_usdc',
      label: 'Swap to Stellar USDC',
      detail: 'Same-chain SDEX / AMM conversion on Stellar.',
      active: true,
    });
  } else {
    steps.push({
      id: 'settle_usdc',
      label: 'Stellar USDC ready',
      detail: asset.isStellarUsdc
        ? 'Already on Stellar — skip the bridge.'
        : 'Mint settles into your Stellar balance before cash-out.',
      active: true,
    });
  }

  steps.push({
    id: 'payout',
    label: 'Bank payout · ₦ Naira',
    detail: 'Partner rail credits your Nigerian bank account.',
    active: true,
  });

  return steps;
}

export function isValidNigerianAccountNumber(value: string): boolean {
  return /^\d{10}$/.test(value.trim());
}
