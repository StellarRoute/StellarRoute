import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { QuoteRefreshLiveRegion } from './QuoteRefreshLiveRegion';

describe('QuoteRefreshLiveRegion', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders polite and assertive live regions without stealing focus', () => {
    render(
      <div>
        <button type="button">Amount</button>
        <QuoteRefreshLiveRegion
          politeMessage="Quote updated. 1 XLM = 0.12 USDC"
          assertiveMessage={null}
        />
      </div>,
    );

    const polite = screen.getByRole('status', { hidden: true });
    const assertive = screen.getByRole('alert', { hidden: true });

    expect(polite).toHaveAttribute('aria-live', 'polite');
    expect(polite).toHaveAttribute('aria-atomic', 'true');
    expect(polite).toHaveClass('sr-only');
    expect(polite).toHaveTextContent('Quote updated. 1 XLM = 0.12 USDC');

    expect(assertive).toHaveAttribute('aria-live', 'assertive');
    expect(assertive).toHaveTextContent('');

    expect(document.activeElement).not.toBe(polite);
    expect(document.activeElement).not.toBe(assertive);
  });

  it('routes hard failures through the assertive region', () => {
    render(
      <QuoteRefreshLiveRegion
        politeMessage={null}
        assertiveMessage="Quote refresh failed. Invalid amount"
      />,
    );

    expect(
      screen.getByText('Quote refresh failed. Invalid amount'),
    ).toHaveAttribute('aria-live', 'assertive');
  });
});
