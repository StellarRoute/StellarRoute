import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import StellarDexAggregatorPage from './page';

describe('StellarDexAggregatorPage', () => {
  it('renders heading, FAQ questions, and does not mount SwapCard', () => {
    render(<StellarDexAggregatorPage />);

    // Assert heading
    const heading = screen.getByRole('heading', { level: 1, name: /Stellar DEX aggregator for SDEX and Soroban/i });
    expect(heading).not.toBeNull();

    // Assert FAQ questions
    expect(screen.getByText('What is a Stellar DEX aggregator?')).not.toBeNull();
    expect(screen.getByText('Does StellarRoute replace the Stellar DEX?')).not.toBeNull();
    expect(screen.getByText('Can I combine DEX aggregation with a cross-chain swap?')).not.toBeNull();

    // Assert that it does not mount SwapCard
    // Assuming SwapCard uses a specific test ID or distinct UI like a swap form, 
    // we just assert it's absent from this marketing page.
    expect(screen.queryByTestId('swap-card')).toBeNull();
  });
});
