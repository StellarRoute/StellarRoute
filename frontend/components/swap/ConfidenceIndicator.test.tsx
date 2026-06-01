import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { ConfidenceIndicator } from './ConfidenceIndicator';

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

describe('ConfidenceIndicator — reduced-motion', () => {
  afterEach(() => setReducedMotion(false));

  it('volatile badge is rendered when volatility=high regardless of motion preference', () => {
    setReducedMotion(true);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    expect(screen.getByTestId('volatile-badge')).toBeInTheDocument();
  });

  it('volatile badge does NOT have animate-pulse when reduced motion is active', () => {
    setReducedMotion(true);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    const badge = screen.getByTestId('volatile-badge');
    expect(badge.className).not.toContain('animate-pulse');
  });

  it('volatile badge HAS animate-pulse when motion is allowed', () => {
    setReducedMotion(false);
    render(<ConfidenceIndicator score={85} volatility="high" />);
    const badge = screen.getByTestId('volatile-badge');
    expect(badge.className).toContain('animate-pulse');
  });

  it('volatile badge is not rendered when volatility is not high', () => {
    setReducedMotion(false);
    render(<ConfidenceIndicator score={85} volatility="low" />);
    expect(screen.queryByTestId('volatile-badge')).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('ConfidenceIndicator — property tests', () => {
  afterEach(() => setReducedMotion(false));

  it(
    // Feature: reduced-motion-swap-animations, Property 11 & 12
    'Property 11 & 12: animate-pulse absent iff prefersReducedMotion is true; badge always present',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          const { unmount } = render(
            <ConfidenceIndicator score={85} volatility="high" />
          );
          const badge = screen.getByTestId('volatile-badge');
          const isPresent = !!badge;
          const hasPulse = badge.className.includes('animate-pulse');
          unmount();

          if (prefersReduced) {
            return isPresent && !hasPulse;
          } else {
            return isPresent && hasPulse;
          }
        }),
        { numRuns: 100 }
      );
    }
  );
});
