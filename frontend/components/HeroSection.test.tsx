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
