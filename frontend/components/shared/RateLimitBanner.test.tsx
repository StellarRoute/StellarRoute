import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { RateLimitBanner } from './RateLimitBanner';

describe('RateLimitBanner', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders an alert role element', () => {
    render(<RateLimitBanner secondsRemaining={5} retry={vi.fn()} />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('displays "Rate limit reached" heading', () => {
    render(<RateLimitBanner secondsRemaining={5} retry={vi.fn()} />);
    expect(screen.getByText('Rate limit reached')).toBeInTheDocument();
  });

  it('shows countdown message with seconds remaining', () => {
    render(<RateLimitBanner secondsRemaining={3} retry={vi.fn()} />);
    expect(
      screen.getByText(/retrying automatically in 3s/i),
    ).toBeInTheDocument();
  });

  it('shows fallback message when secondsRemaining is 0', () => {
    render(<RateLimitBanner secondsRemaining={0} retry={vi.fn()} />);
    expect(
      screen.getByText(/please wait a moment and try again/i),
    ).toBeInTheDocument();
  });

  it('does not show raw stack traces or HTTP status codes', () => {
    render(<RateLimitBanner secondsRemaining={5} retry={vi.fn()} />);
    const alertEl = screen.getByRole('alert');
    expect(alertEl.textContent).not.toMatch(/429/);
    expect(alertEl.textContent).not.toMatch(/Error:/i);
    expect(alertEl.textContent).not.toMatch(/at /);
  });

  it('shows the countdown number in the progress ring', () => {
    render(<RateLimitBanner secondsRemaining={7} retry={vi.fn()} />);
    expect(screen.getByText('7')).toBeInTheDocument();
  });

  it('hides progress ring when secondsRemaining is 0', () => {
    render(<RateLimitBanner secondsRemaining={0} retry={vi.fn()} />);
    expect(screen.queryByText('0')).not.toBeInTheDocument();
    // SVG should not be present
    const alertEl = screen.getByRole('alert');
    expect(alertEl.querySelector('svg.h-8')).not.toBeInTheDocument();
  });

  it('renders a Retry Now button', () => {
    render(<RateLimitBanner secondsRemaining={5} retry={vi.fn()} />);
    expect(
      screen.getByRole('button', { name: /retry now/i }),
    ).toBeInTheDocument();
  });

  it('calls retry callback when Retry Now is clicked', async () => {
    const retry = vi.fn();
    const user = userEvent.setup();
    render(<RateLimitBanner secondsRemaining={5} retry={retry} />);

    await user.click(screen.getByRole('button', { name: /retry now/i }));

    expect(retry).toHaveBeenCalledTimes(1);
  });
});
