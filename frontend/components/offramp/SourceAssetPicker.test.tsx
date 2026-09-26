import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

import { SourceAssetPicker } from './SourceAssetPicker';
import type { OfframpSourceAsset } from '@/lib/offramp/types';

const FIXTURE_ASSETS: OfframpSourceAsset[] = [
  {
    id: 'stellar-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Stellar',
    kind: 'stellar_usdc',
    status: 'ready',
    isStellarUsdc: true,
    decimals: 7,
    hint: 'Offramp directly — no bridge step.',
  },
  {
    id: 'eth-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Ethereum',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge to Stellar USDC, then cash out to Naira.',
  },
];

describe('SourceAssetPicker', () => {
  it('renders every fixture asset', () => {
    render(
      <SourceAssetPicker
        assets={FIXTURE_ASSETS}
        selectedId="stellar-usdc"
        onSelect={vi.fn()}
      />,
    );

    expect(screen.getByTestId('offramp-source-picker')).toBeInTheDocument();
    for (const asset of FIXTURE_ASSETS) {
      expect(screen.getByTestId(`offramp-asset-${asset.id}`)).toBeInTheDocument();
    }
  });

  it('keeps EVM-only sources listed alongside Stellar sources', () => {
    render(
      <SourceAssetPicker
        assets={FIXTURE_ASSETS}
        selectedId="stellar-usdc"
        onSelect={vi.fn()}
      />,
    );

    const evmRow = screen.getByTestId('offramp-asset-eth-usdc');
    expect(evmRow).toBeInTheDocument();
    expect(evmRow).toHaveTextContent('Ethereum');
  });

  it('does not claim Stellar is a Paycrest deposit network', () => {
    const { container } = render(
      <SourceAssetPicker
        assets={FIXTURE_ASSETS}
        selectedId="stellar-usdc"
        onSelect={vi.fn()}
      />,
    );

    expect(container.textContent ?? '').not.toMatch(/paycrest/i);
    expect(container.textContent ?? '').not.toMatch(/deposit network/i);
  });
});
