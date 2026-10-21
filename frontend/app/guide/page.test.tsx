import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import FirstSwapGuidePage from './page';

// ---------------------------------------------------------------------------
// The guide page is a pure RSC-style server component with no hooks, no
// wallet dependency, and no dynamic imports — it renders synchronously in
// jsdom without any mocking.
// ---------------------------------------------------------------------------

describe('FirstSwapGuidePage', () => {
  it('renders without throwing and without wallet connection', () => {
    // Must not throw even when wallet context is absent.
    render(<FirstSwapGuidePage />);
    expect(screen.getByRole('main')).toBeInTheDocument();
  });

  it('renders the primary heading', () => {
    render(<FirstSwapGuidePage />);
    expect(
      screen.getByRole('heading', { level: 1, name: /your first live swap/i }),
    ).toBeInTheDocument();
  });

  it('renders the "Open swap" CTA link pointing to /swap', () => {
    render(<FirstSwapGuidePage />);
    const link = screen.getByRole('link', { name: /open swap/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute('href', '/swap');
  });

  it('renders the "Full guide on GitHub" external link', () => {
    render(<FirstSwapGuidePage />);
    const link = screen.getByRole('link', { name: /full guide on github/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute(
      'href',
      'https://github.com/StellarRoute/StellarRoute/blob/main/docs/user-guide-first-live-swap.md',
    );
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('renders the risk-disclosure external link', () => {
    render(<FirstSwapGuidePage />);
    const link = screen.getByRole('link', { name: /risk disclosure/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute(
      'href',
      'https://github.com/StellarRoute/StellarRoute/blob/main/docs/risk-disclosure.md',
    );
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('renders all 6 numbered steps', () => {
    render(<FirstSwapGuidePage />);
    // Steps are rendered inside an <ol> as <li> elements.
    const items = screen.getAllByRole('listitem');
    expect(items.length).toBeGreaterThanOrEqual(6);
  });

  it('renders step headings covering the first-swap journey', () => {
    render(<FirstSwapGuidePage />);
    expect(
      screen.getByRole('heading', { name: /connect your wallet/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /fund and reserve xlm/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /add a trustline/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /pick a pair/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /set slippage/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: /confirm in your wallet/i }),
    ).toBeInTheDocument();
  });

  it('renders the "Before you confirm" risk aside', () => {
    render(<FirstSwapGuidePage />);
    expect(screen.getByText(/before you confirm/i)).toBeInTheDocument();
  });

  it('displays the "User guide" section label', () => {
    render(<FirstSwapGuidePage />);
    expect(screen.getByText(/user guide/i)).toBeInTheDocument();
  });

  it('describes the non-custodial key model in step 6 body copy', () => {
    render(<FirstSwapGuidePage />);
    expect(
      screen.getByText(/StellarRoute never holds your keys/i),
    ).toBeInTheDocument();
  });
});
