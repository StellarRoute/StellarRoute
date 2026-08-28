import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { OfframpDashboard } from './OfframpDashboard';

describe('OfframpDashboard', () => {
  it('renders the cash corridor hero and default direct path', () => {
    render(<OfframpDashboard />);
    expect(screen.getByTestId('offramp-dashboard')).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /stablecoin to local fiat/i }),
    ).toBeInTheDocument();
    expect(screen.getByTestId('offramp-mode-direct')).toHaveAttribute(
      'aria-checked',
      'true',
    );
  });

  it('switches to bridge mode and unlocks non-Stellar assets', async () => {
    const user = userEvent.setup();
    render(<OfframpDashboard />);

    await user.click(screen.getByTestId('offramp-mode-bridge'));
    expect(screen.getByTestId('offramp-mode-bridge')).toHaveAttribute(
      'aria-checked',
      'true',
    );

    await user.click(screen.getByTestId('offramp-asset-eth-usdc'));
    expect(screen.getByTestId('offramp-route-rail')).toHaveTextContent(
      /Bridge to Stellar/i,
    );
  });

  it('previews an indicative Naira quote after amount entry', async () => {
    const user = userEvent.setup();
    render(<OfframpDashboard />);

    await user.type(screen.getByTestId('offramp-amount'), '100');
    expect(screen.getByTestId('offramp-quote-summary')).toHaveTextContent('₦');
    expect(screen.getByTestId('offramp-quote-summary')).toHaveTextContent(
      '157,210.00',
    );
  });
});
