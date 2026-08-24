import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { IconographyLegend } from '@/components/shared/IconographyLegend';

vi.mock('@/lib/swap-i18n', () => ({
  useSwapI18n: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'swap.iconography.disclosure': 'Route and transaction icon legend',
        'swap.iconography.eyebrow': 'Iconography System',
        'swap.iconography.title': 'Route and Transaction Icons',
        'swap.iconography.description':
          'Consistent icons help users distinguish between venue types, hybrid routes, and transaction lifecycle states.',
        'swap.iconography.venueTypes': 'Venue Types',
        'swap.iconography.venueTypes.sdex':
          'SDEX represents order book trades. AMM indicates liquidity pool swaps.',
        'swap.iconography.venueTypes.hybrid':
          'Hybrid routes combine both venue types for optimal routing.',
        'swap.iconography.transactionStates': 'Transaction States',
        'swap.iconography.sizingNote':
          'Icons are sized for screen readability at 16/20/24px.',
        'swap.iconography.assetFallbackNote':
          'Asset icons fall back to stable uppercase initials when a valid image source is unavailable.',
      };
      return translations[key] ?? key;
    },
  }),
}));

describe('IconographyLegend', () => {
  it('renders venue and transaction badges from shared components', () => {
    render(<IconographyLegend />);

    expect(screen.getByText('SDEX')).toBeInTheDocument();
    expect(screen.getByText('AMM')).toBeInTheDocument();
    expect(screen.getByText('Hybrid')).toBeInTheDocument();
    expect(screen.getByText('Pending')).toBeInTheDocument();
    expect(screen.getByText('Confirmed')).toBeInTheDocument();
  });

  it('expands legend content through keyboard-accessible disclosure', async () => {
    render(<IconographyLegend />);
    const user = userEvent.setup();
    const details = screen.getByTestId('iconography-legend');

    expect(details).not.toHaveAttribute('open');

    await user.click(screen.getByText('Route and transaction icon legend'));

    expect(details).toHaveAttribute('open');
    expect(screen.getByText('Route and Transaction Icons')).toBeInTheDocument();
    expect(screen.getByText('Venue Types')).toBeInTheDocument();
    expect(screen.getByText('Transaction States')).toBeInTheDocument();
  });
});
