import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { SourceAssetPicker } from './SourceAssetPicker';
import {
  OFFRAMP_SOURCE_ASSETS,
  findOfframpSource,
  resolveOfframpMode,
} from '@/lib/offramp/assets';
import type { OfframpSourceAsset } from '@/lib/offramp/types';

// ---------------------------------------------------------------------------
// Fixture assets
// ---------------------------------------------------------------------------

/** Minimal fixture set covering every status variant in the real asset list. */
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
    id: 'stellar-xlm',
    symbol: 'XLM',
    name: 'Lumens',
    chainLabel: 'Stellar',
    kind: 'stellar_xlm',
    status: 'swap_then_offramp',
    isStellarUsdc: false,
    decimals: 7,
    hint: 'Swap to Stellar USDC, then cash out to Naira.',
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
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'base-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Base',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'arb-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Arbitrum',
    kind: 'evm_usdc',
    status: 'bridge_required',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Bridge via Circle CCTP → Stellar USDC → Naira.',
  },
  {
    id: 'sol-usdc',
    symbol: 'USDC',
    name: 'USD Coin',
    chainLabel: 'Solana',
    kind: 'solana_usdc',
    status: 'coming_soon',
    isStellarUsdc: false,
    decimals: 6,
    hint: 'Solana CCTP corridor coming soon.',
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderPicker(
  overrides: Partial<Parameters<typeof SourceAssetPicker>[0]> = {},
) {
  const onSelect = vi.fn();
  render(
    <SourceAssetPicker
      assets={FIXTURE_ASSETS}
      selectedId="stellar-usdc"
      onSelect={onSelect}
      {...overrides}
    />,
  );
  return { onSelect };
}

// ---------------------------------------------------------------------------
// 1. Fixture assets render
// ---------------------------------------------------------------------------

describe('SourceAssetPicker — fixture assets render', () => {
  it('renders the picker container', () => {
    renderPicker();
    expect(screen.getByTestId('offramp-source-picker')).toBeInTheDocument();
  });

  it('renders a button for every fixture asset', () => {
    renderPicker();
    for (const asset of FIXTURE_ASSETS) {
      expect(
        screen.getByTestId(`offramp-asset-${asset.id}`),
      ).toBeInTheDocument();
    }
  });

  it('marks the selected asset as aria-selected=true', () => {
    renderPicker({ selectedId: 'eth-usdc' });
    const ethBtn = screen.getByTestId('offramp-asset-eth-usdc');
    expect(ethBtn).toHaveAttribute('aria-selected', 'true');
  });

  it('marks non-selected assets as aria-selected=false', () => {
    renderPicker({ selectedId: 'eth-usdc' });
    for (const asset of FIXTURE_ASSETS.filter((a) => a.id !== 'eth-usdc')) {
      expect(screen.getByTestId(`offramp-asset-${asset.id}`)).toHaveAttribute(
        'aria-selected',
        'false',
      );
    }
  });

  it('shows the chain label for each asset', () => {
    renderPicker();
    expect(screen.getByText('Ethereum')).toBeInTheDocument();
    expect(screen.getByText('Base')).toBeInTheDocument();
    expect(screen.getByText('Arbitrum')).toBeInTheDocument();
  });

  it('shows status badges (Ready / Bridge / Swap / Soon)', () => {
    renderPicker();
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getAllByText('Bridge').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Swap')).toBeInTheDocument();
    expect(screen.getByText('Soon')).toBeInTheDocument();
  });

  it('shows the hint text for each asset', () => {
    renderPicker();
    expect(
      screen.getByText('Offramp directly — no bridge step.'),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText('Bridge via Circle CCTP → Stellar USDC → Naira.')
        .length,
    ).toBeGreaterThanOrEqual(1);
  });

  it('renders with the real OFFRAMP_SOURCE_ASSETS list without throwing', () => {
    // Smoke-test: the production asset list should render without error.
    render(
      <SourceAssetPicker
        assets={OFFRAMP_SOURCE_ASSETS}
        selectedId="stellar-usdc"
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByTestId('offramp-source-picker')).toBeInTheDocument();
    // All 6 production assets should appear.
    expect(
      screen.getAllByRole('option').length,
    ).toBe(OFFRAMP_SOURCE_ASSETS.length);
  });
});

// ---------------------------------------------------------------------------
// 2. Interaction — onSelect callback
// ---------------------------------------------------------------------------

describe('SourceAssetPicker — interaction', () => {
  it('calls onSelect with the asset id when an enabled button is clicked', async () => {
    const user = userEvent.setup();
    const { onSelect } = renderPicker({ selectedId: 'stellar-usdc' });

    await user.click(screen.getByTestId('offramp-asset-eth-usdc'));
    expect(onSelect).toHaveBeenCalledWith('eth-usdc');
  });

  it('does not call onSelect for a coming_soon asset', async () => {
    const user = userEvent.setup();
    const { onSelect } = renderPicker({ selectedId: 'stellar-usdc' });

    const solBtn = screen.getByTestId('offramp-asset-sol-usdc');
    expect(solBtn).toBeDisabled();

    // userEvent.click on a disabled button should be a no-op.
    await user.click(solBtn);
    expect(onSelect).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// 3. directOnly mode
// ---------------------------------------------------------------------------

describe('SourceAssetPicker — directOnly mode', () => {
  it('shows the direct-only description copy', () => {
    renderPicker({ directOnly: true });
    expect(
      screen.getByText(/Direct path uses Stellar USDC only/i),
    ).toBeInTheDocument();
  });

  it('only allows Stellar USDC to be clicked in directOnly mode', () => {
    renderPicker({ directOnly: true });
    // Only stellar-usdc should be enabled.
    expect(screen.getByTestId('offramp-asset-stellar-usdc')).not.toBeDisabled();
    // All others must be disabled.
    for (const asset of FIXTURE_ASSETS.filter((a) => a.id !== 'stellar-usdc')) {
      expect(
        screen.getByTestId(`offramp-asset-${asset.id}`),
      ).toBeDisabled();
    }
  });

  it('calls onSelect when Stellar USDC is clicked in directOnly mode', async () => {
    const user = userEvent.setup();
    const { onSelect } = renderPicker({
      directOnly: true,
      selectedId: 'stellar-usdc',
    });

    await user.click(screen.getByTestId('offramp-asset-stellar-usdc'));
    expect(onSelect).toHaveBeenCalledWith('stellar-usdc');
  });
});

// ---------------------------------------------------------------------------
// 4. Paycrest deposit-network assertions
//    Stellar is NOT a Paycrest deposit network — EVM chains (ETH, Base, ARB)
//    are the direct Paycrest sources. The picker must not claim otherwise.
// ---------------------------------------------------------------------------

describe('OFFRAMP_SOURCE_ASSETS — Paycrest deposit network correctness', () => {
  it('EVM USDC assets use bridge_required status (Paycrest deposit networks)', () => {
    const evmAssets = OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.kind === 'evm_usdc',
    );
    // ETH, Base, Arbitrum must all be present.
    expect(evmAssets.length).toBeGreaterThanOrEqual(3);

    for (const asset of evmAssets) {
      expect(asset.status).toBe('bridge_required');
    }
  });

  it('Stellar assets do NOT carry isStellarUsdc=true on more than one asset', () => {
    const directAssets = OFFRAMP_SOURCE_ASSETS.filter((a) => a.isStellarUsdc);
    // There should be exactly one direct Stellar USDC asset.
    expect(directAssets).toHaveLength(1);
    expect(directAssets[0].id).toBe('stellar-usdc');
  });

  it('Stellar-native assets are NOT listed as bridge_required (they do not go through Paycrest EVM deposit)', () => {
    const stellarAssets = OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.kind === 'stellar_usdc' || a.kind === 'stellar_xlm',
    );
    for (const asset of stellarAssets) {
      expect(asset.status).not.toBe('bridge_required');
    }
  });

  it('resolveOfframpMode returns "direct" only for stellar-usdc', () => {
    const directAsset = findOfframpSource('stellar-usdc')!;
    expect(resolveOfframpMode(directAsset)).toBe('direct');

    for (const asset of OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.id !== 'stellar-usdc',
    )) {
      expect(resolveOfframpMode(asset)).toBe('bridge');
    }
  });

  it('hint copy for Stellar-native assets does not claim Stellar is a Paycrest deposit network', () => {
    const stellarAssets = OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.kind === 'stellar_usdc' || a.kind === 'stellar_xlm',
    );
    for (const asset of stellarAssets) {
      // Paycrest deposit networks are EVM; Stellar hint must not say "Paycrest deposit"
      expect(asset.hint.toLowerCase()).not.toMatch(/paycrest deposit/);
    }
  });

  it('hint copy for EVM assets mentions bridge (they bridge into Stellar USDC first)', () => {
    const evmAssets = OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.kind === 'evm_usdc',
    );
    for (const asset of evmAssets) {
      expect(asset.hint.toLowerCase()).toMatch(/bridge/);
    }
  });

  it('no Stellar asset has kind=evm_usdc', () => {
    const stellar = OFFRAMP_SOURCE_ASSETS.filter(
      (a) => a.chainLabel.toLowerCase() === 'stellar',
    );
    for (const asset of stellar) {
      expect(asset.kind).not.toBe('evm_usdc');
    }
  });
});

// ---------------------------------------------------------------------------
// 5. Accessibility
// ---------------------------------------------------------------------------

describe('SourceAssetPicker — accessibility', () => {
  it('wraps options in a listbox role', () => {
    renderPicker();
    expect(screen.getByRole('listbox')).toBeInTheDocument();
  });

  it('listbox has an accessible label', () => {
    renderPicker();
    expect(
      screen.getByRole('listbox', { name: /source asset/i }),
    ).toBeInTheDocument();
  });

  it('each asset is exposed as an option role', () => {
    renderPicker();
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(FIXTURE_ASSETS.length);
  });
});
