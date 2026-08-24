import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { HeroSection } from './HeroSection';

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function setReducedMotion(value: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: value,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(() => false),
    }),
  });
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

describe('HeroSection — reduced-motion', () => {
  afterEach(() => setReducedMotion(false));

  it('gradient divs do NOT have animate-pulse when reduced motion is active', () => {
    setReducedMotion(true);
    render(<HeroSection />);
    const g1 = screen.getByTestId('hero-gradient-1');
    const g2 = screen.getByTestId('hero-gradient-2');
    expect(g1.className).not.toContain('animate-pulse');
    expect(g2.className).not.toContain('animate-pulse');
  });

  it('gradient divs HAVE animate-pulse when motion is allowed', () => {
    setReducedMotion(false);
    render(<HeroSection />);
    const g1 = screen.getByTestId('hero-gradient-1');
    const g2 = screen.getByTestId('hero-gradient-2');
    expect(g1.className).toContain('animate-pulse');
    expect(g2.className).toContain('animate-pulse');
  });

  it('both gradient divs are always rendered regardless of motion preference', () => {
    setReducedMotion(true);
    render(<HeroSection />);
    expect(screen.getByTestId('hero-gradient-1')).toBeInTheDocument();
    expect(screen.getByTestId('hero-gradient-2')).toBeInTheDocument();
  });

  it('does not render motion classes when reduced motion is active', () => {
    setReducedMotion(true);
    const { container } = render(<HeroSection />);
    expect(container.querySelectorAll('[class*="animate-"]')).toHaveLength(0);
  });
});

describe('HeroSection — product positioning', () => {
  afterEach(() => setReducedMotion(false));

  it('positions StellarRoute as a non-custodial Stellar DEX aggregator', () => {
    render(<HeroSection />);
    expect(
      screen.getByRole('heading', {
        level: 1,
        name: /Stellar DEX aggregator\. Cross-chain swaps beyond it\./i,
      })
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Non-custodial Stellar DEX aggregator/i)
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(/Stellar DEX \(SDEX\) and Soroban AMMs/i)
    ).not.toHaveLength(0);
    expect(
      screen.getByRole('link', { name: /Stellar DEX aggregator/i })
    ).toHaveAttribute('href', '/stellar-dex-aggregator');
    expect(
      screen.getByRole('link', { name: /Cross-chain swap/i })
    ).toHaveAttribute('href', '/cross-chain-swap');
  });

  it('links both primary calls to action directly to the swap deck', () => {
    render(<HeroSection />);
    const links = screen.getAllByRole('link', {
      name: /Open execution deck/i,
    });
    expect(links).toHaveLength(2);
    links.forEach((link) => expect(link).toHaveAttribute('href', '/swap'));
  });

  it('labels the live corridor as testnet and states its limits', () => {
    render(<HeroSection />);
    expect(screen.getAllByText('TESTNET CORRIDOR')).not.toHaveLength(0);
    expect(screen.getAllByText(/Stellar ↔ Ethereum Sepolia/i)).not.toHaveLength(0);
    expect(
      screen.getByText(/CCTP is disabled by default at the API layer/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/does not claim mainnet availability/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Both directions are signed-live on testnet/i)
    ).toBeInTheDocument();
  });

  it('shows the signed-live testnet evidence metrics', () => {
    render(<HeroSection />);
    expect(screen.getByText('Signed-live / testnet evidence')).toBeInTheDocument();
    expect(screen.getByText('Total saga')).toBeInTheDocument();
    expect(screen.getByText('Burn → attestation')).toBeInTheDocument();
    expect(screen.getByText('63')).toBeInTheDocument();
    expect(screen.getByText('33')).toBeInTheDocument();
    expect(
      screen.getByRole('link', {
        name: /0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6/i,
      })
    ).toHaveAttribute(
      'href',
      'https://sepolia.etherscan.io/tx/0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6'
    );
    expect(
      screen.getByRole('link', {
        name: /13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2/i,
      })
    ).toHaveAttribute(
      'href',
      'https://stellar.expert/explorer/testnet/tx/13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2'
    );
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('HeroSection — property tests', () => {
  afterEach(() => setReducedMotion(false));

  it(
    // Feature: reduced-motion-swap-animations, Property 13 & 14
    'Property 13 & 14: animate-pulse absent iff prefersReducedMotion is true on both gradient divs',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          const { unmount } = render(<HeroSection />);
          const g1 = screen.getByTestId('hero-gradient-1');
          const g2 = screen.getByTestId('hero-gradient-2');
          const g1HasPulse = g1.className.includes('animate-pulse');
          const g2HasPulse = g2.className.includes('animate-pulse');
          unmount();

          if (prefersReduced) {
            return !g1HasPulse && !g2HasPulse;
          } else {
            return g1HasPulse && g2HasPulse;
          }
        }),
        { numRuns: 100 }
      );
    }
  );
});
