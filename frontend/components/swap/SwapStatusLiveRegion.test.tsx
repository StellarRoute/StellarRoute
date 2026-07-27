import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { SwapStatusLiveRegion } from './SwapStatusLiveRegion';

describe('SwapStatusLiveRegion', () => {
  afterEach(() => {
    cleanup();
  });

  it('announces pending politely', () => {
    render(<SwapStatusLiveRegion status="pending" />);

    const polite = screen.getByRole('status', { hidden: true });
    expect(polite).toHaveAttribute('aria-live', 'polite');
    expect(polite).toHaveTextContent('Swap submitted. Waiting for confirmation.');
  });

  it('announces finalized politely with detail', () => {
    render(<SwapStatusLiveRegion status="finalized" detail="Transaction abc123." />);

    expect(screen.getByRole('status', { hidden: true })).toHaveTextContent(
      'Swap confirmed. Transaction abc123.',
    );
  });

  it('announces errors assertively', () => {
    render(<SwapStatusLiveRegion status="error" detail="Slippage exceeded." />);

    const alert = screen.getByRole('alert', { hidden: true });
    expect(alert).toHaveAttribute('aria-live', 'assertive');
    expect(alert).toHaveTextContent('Swap failed. Slippage exceeded.');
  });

  it('exposes a keyboard path to update the quote and confirm', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onUpdateQuote = vi.fn();

    render(
      <SwapStatusLiveRegion status="error" onConfirm={onConfirm} onUpdateQuote={onUpdateQuote} />,
    );

    await user.tab();
    expect(screen.getByRole('button', { name: /update quote/i })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onUpdateQuote).toHaveBeenCalledTimes(1);

    await user.tab();
    expect(screen.getByRole('button', { name: /confirm swap/i })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it('disables actions while a swap is pending', () => {
    render(<SwapStatusLiveRegion status="pending" onConfirm={vi.fn()} onUpdateQuote={vi.fn()} />);

    expect(screen.getByRole('button', { name: /confirm swap/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /update quote/i })).toBeDisabled();
  });
});
