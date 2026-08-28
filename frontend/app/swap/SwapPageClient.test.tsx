import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { SwapPageClient } from './SwapPageClient';

vi.mock('next/dynamic', () => ({
  default: () => {
    const MockSwapCard = () => <div data-testid="swap-card">SwapCard</div>;
    return MockSwapCard;
  },
}));

vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: vi.fn(),
}));

vi.mock('@/components/swap/cross-chain/CrossChainSwapDeck', () => ({
  CrossChainSwapDeck: () => <div data-testid="cross-chain-swap-deck">deck</div>,
}));

vi.mock('@/hooks/useSplitView', () => ({
  useSplitView: () => ({ isSplit: false, toggleSplit: vi.fn() }),
}));

import { useFeatureFlag } from '@/hooks/useFeatureFlag';

describe('SwapPageClient swap_ui_v2 gate', () => {
  beforeEach(() => {
    vi.mocked(useFeatureFlag).mockImplementation((flag) => {
      if (flag === 'swap_ui_v2') {
        return { enabled: false, loading: false };
      }
      if (flag === 'routes_beta') {
        return { enabled: false, loading: false };
      }
      return { enabled: false, loading: false };
    });
  });

  it('renders legacy swap card when swap_ui_v2 is disabled', () => {
    render(<SwapPageClient />);
    expect(screen.getByTestId('swap-card')).toBeInTheDocument();
    expect(screen.queryByTestId('cross-chain-swap-deck')).not.toBeInTheDocument();
  });

  it('renders legacy surface while swap_ui_v2 is loading', () => {
    vi.mocked(useFeatureFlag).mockImplementation((flag) => {
      if (flag === 'swap_ui_v2') {
        return { enabled: false, loading: true };
      }
      return { enabled: false, loading: false };
    });
    render(<SwapPageClient />);
    expect(screen.getByTestId('swap-card')).toBeInTheDocument();
    expect(screen.queryByTestId('cross-chain-swap-deck')).not.toBeInTheDocument();
    expect(screen.queryByTestId('cross-chain-swap-deck-skeleton')).not.toBeInTheDocument();
  });

  it('renders cross-chain deck when swap_ui_v2 is enabled', () => {
    vi.mocked(useFeatureFlag).mockImplementation((flag) => {
      if (flag === 'swap_ui_v2') {
        return { enabled: true, loading: false };
      }
      return { enabled: false, loading: false };
    });
    render(<SwapPageClient />);
    expect(screen.getByTestId('cross-chain-swap-deck')).toBeInTheDocument();
    expect(screen.queryByTestId('swap-card')).not.toBeInTheDocument();
  });
});
