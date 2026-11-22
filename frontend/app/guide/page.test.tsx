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

  // ── Table of contents + skip link (issue #1735) ───────────────────────────

  it('renders a "Skip to steps" skip link targeting #guide-steps', () => {
    render(<FirstSwapGuidePage />);
    const skipLink = screen.getByRole('link', { name: /skip to steps/i });
    expect(skipLink).toBeInTheDocument();
    expect(skipLink).toHaveAttribute('href', '#guide-steps');
  });

  it('renders a TOC nav landmark with accessible label', () => {
    render(<FirstSwapGuidePage />);
    const toc = screen.getByRole('navigation', { name: /guide table of contents/i });
    expect(toc).toBeInTheDocument();
  });

  it('renders TOC links for all 6 steps', () => {
    render(<FirstSwapGuidePage />);
    const toc = screen.getByTestId('guide-toc');
    const tocLinks = toc.querySelectorAll('a');
    expect(tocLinks).toHaveLength(6);
  });

  it('renders TOC anchor links with correct fragment hrefs', () => {
    render(<FirstSwapGuidePage />);
    const connectLink = screen.getByRole('link', { name: /1\.\s*Connect your wallet/i });
    expect(connectLink).toHaveAttribute('href', '#step-connect-wallet');

    const confirmLink = screen.getByRole('link', { name: /6\.\s*Confirm in your wallet/i });
    expect(confirmLink).toHaveAttribute('href', '#step-confirm-wallet');
  });

  it('each step li has an id matching the TOC fragment', () => {
    const { container } = render(<FirstSwapGuidePage />);
    const stepList = container.querySelector('#guide-steps');
    expect(stepList).toBeInTheDocument();

    const stepIds = Array.from(stepList!.querySelectorAll('li[id]')).map(
      (el) => el.getAttribute('id'),
    );
    expect(stepIds).toContain('step-connect-wallet');
    expect(stepIds).toContain('step-confirm-wallet');
    expect(stepIds).toHaveLength(6);
  });
});
