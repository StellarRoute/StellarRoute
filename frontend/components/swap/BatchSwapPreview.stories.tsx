import { useEffect, useState } from 'react';
import { BatchSwapPreview, BatchSwapLeg } from './BatchSwapPreview';

// ── Fixture data ────────────────────────────────────────────────────────────

const twoLegFixture: BatchSwapLeg[] = [
  {
    id: 'leg-1',
    fromAsset: 'XLM',
    toAsset: 'USDC',
    fromAmount: '500.00',
    toAmount: '52.50',
    price: '0.105000',
    priceImpact: '0.08',
  },
  {
    id: 'leg-2',
    fromAsset: 'USDC',
    toAsset: 'BTC',
    fromAmount: '52.50',
    toAmount: '0.000875',
    price: '0.000016667',
    priceImpact: '0.21',
  },
];

const highImpactFixture: BatchSwapLeg[] = [
  {
    id: 'leg-hi-1',
    fromAsset: 'XLM',
    toAsset: 'AQUA',
    fromAmount: '250000.00',
    toAmount: '8750000.00',
    price: '35.000000',
    priceImpact: '4.72',
  },
  {
    id: 'leg-hi-2',
    fromAsset: 'AQUA',
    toAsset: 'USDC',
    fromAmount: '8750000.00',
    toAmount: '875.00',
    price: '0.000100',
    priceImpact: '6.15',
  },
];

const fiveLegsFixture: BatchSwapLeg[] = [
  { id: 'l1', fromAsset: 'XLM',  toAsset: 'USDC', fromAmount: '100', toAmount: '10.50',  price: '0.105', priceImpact: '0.05' },
  { id: 'l2', fromAsset: 'USDC', toAsset: 'ETH',  fromAmount: '10.50', toAmount: '0.0035', price: '0.000333', priceImpact: '0.11' },
  { id: 'l3', fromAsset: 'ETH',  toAsset: 'BTC',  fromAmount: '0.0035', toAmount: '0.000058', price: '0.016571', priceImpact: '0.09' },
  { id: 'l4', fromAsset: 'BTC',  toAsset: 'USDC', fromAmount: '0.000058', toAmount: '3.48', price: '60000', priceImpact: '0.02' },
  { id: 'l5', fromAsset: 'USDC', toAsset: 'XLM',  fromAmount: '3.48', toAmount: '33.14', price: '9.523', priceImpact: '0.04' },
];

// ── Storybook meta ──────────────────────────────────────────────────────────

export default {
  title: 'Swap / BatchSwapPreview',
};

// Enable the feature flag in every story via runtime config
function enableBatchSwaps() {
  (window as any).__STELLAR_ROUTE_FLAGS__ = { batchSwaps: true };
}

// ── Stories ─────────────────────────────────────────────────────────────────

/** Default two-leg XLM → USDC → BTC preview */
export const DefaultTwoLegs = () => {
  enableBatchSwaps();
  const [legs, setLegs] = useState<BatchSwapLeg[]>(twoLegFixture);
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview
        legs={legs}
        onConfirm={() => alert('Batch submitted!')}
        onCancel={() => setLegs([])}
      />
    </div>
  );
};

/** Five-leg cross-asset cycle */
export const FiveLegs = () => {
  enableBatchSwaps();
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview
        legs={fiveLegsFixture}
        onConfirm={() => alert('Batch submitted!')}
        onCancel={() => {}}
      />
    </div>
  );
};

/** Legs with high price-impact warnings */
export const HighImpactWarnings = () => {
  enableBatchSwaps();
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview
        legs={highImpactFixture}
        onConfirm={() => alert('Batch submitted!')}
        onCancel={() => {}}
      />
    </div>
  );
};

/** Loading skeleton while simulation is in progress */
export const Loading = () => {
  enableBatchSwaps();
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview legs={twoLegFixture} isLoading />
    </div>
  );
};

/** Empty state — no legs added yet */
export const Empty = () => {
  enableBatchSwaps();
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview legs={[]} onConfirm={() => {}} onCancel={() => {}} />
    </div>
  );
};

/** Error state — backend simulation failed */
export const SimulationError = () => {
  enableBatchSwaps();
  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview
        legs={[]}
        error="Liquidity pool returned an unexpected error. Please try again or reduce your trade size."
        onRetry={() => alert('Retrying…')}
      />
    </div>
  );
};

/** Feature flag disabled — shows the Beta gate card */
export const FeatureFlagDisabled = () => {
  useEffect(() => {
    (window as any).__STELLAR_ROUTE_FLAGS__ = { batchSwaps: false };
  }, []);

  return (
    <div className="max-w-md mx-auto p-6">
      <BatchSwapPreview legs={twoLegFixture} />
    </div>
  );
};
