import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { SwapButton, type SwapButtonState } from './SwapButton';

// ---------------------------------------------------------------------------
// Helper: override window.matchMedia per-test
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

const noop = () => {};

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

describe('SwapButton — reduced-motion', () => {
  afterEach(() => setReducedMotion(false));

  // --- high_impact_warning ---

  it('omits animate-pulse from high-impact button when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="high_impact_warning" onSwap={noop} />);
    const btn = screen.getByRole('button');
    expect(btn.className).not.toContain('animate-pulse');
  });

  it('includes animate-pulse on high-impact button when motion is allowed', () => {
    setReducedMotion(false);
    render(<SwapButton state="high_impact_warning" onSwap={noop} />);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('animate-pulse');
  });

  it('omits animate-spin from Loader2 in high_impact_warning+loading when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="high_impact_warning" onSwap={noop} isLoading />);
    // The lucide mock renders all icons as <svg data-testid="icon">
    const icons = document.querySelectorAll('[data-testid="icon"]');
    icons.forEach((icon) => {
      expect(icon.className).not.toContain('animate-spin');
    });
  });

  // --- ready ---

  it('omits active:scale-[0.98] and transition-all from ready button when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="ready" onSwap={noop} />);
    const btn = screen.getByRole('button');
    expect(btn.className).not.toContain('active:scale-[0.98]');
    expect(btn.className).not.toContain('transition-all');
  });

  it('omits transition-all duration-300 from button container when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="ready" onSwap={noop} />);
    const btn = screen.getByRole('button');
    expect(btn.className).not.toContain('duration-300');
  });

  // --- executing ---

  it('includes animate-spin on Loader2 in executing state when motion is allowed', () => {
    setReducedMotion(false);
    render(<SwapButton state="executing" onSwap={noop} />);
    const icons = document.querySelectorAll('[data-testid="icon"]');
    const spinners = Array.from(icons).filter((el) =>
      el.className.includes('animate-spin')
    );
    expect(spinners.length).toBeGreaterThan(0);
  });

  it('omits animate-spin from Loader2 in executing state when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="executing" onSwap={noop} />);
    const icons = document.querySelectorAll('[data-testid="icon"]');
    icons.forEach((icon) => {
      expect(icon.className).not.toContain('animate-spin');
    });
  });

  // --- refreshing_quote ---

  it('omits animate-spin from Loader2 in refreshing_quote state when reduced motion is active', () => {
    setReducedMotion(true);
    render(<SwapButton state="refreshing_quote" onSwap={noop} />);
    const icons = document.querySelectorAll('[data-testid="icon"]');
    icons.forEach((icon) => {
      expect(icon.className).not.toContain('animate-spin');
    });
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

const LOADING_STATES: SwapButtonState[] = [
  'executing',
  'refreshing_quote',
  'high_impact_warning',
];

describe('SwapButton — property tests', () => {
  afterEach(() => setReducedMotion(false));

  it(
    // Feature: reduced-motion-swap-animations, Property 5 & 6
    'Property 5 & 6: animate-spin absent iff prefersReducedMotion is true (loading states)',
    () => {
      fc.assert(
        fc.property(
          fc.boolean(),
          fc.constantFrom(...LOADING_STATES),
          (prefersReduced, state) => {
            setReducedMotion(prefersReduced);
            const { unmount } = render(
              <SwapButton state={state} onSwap={noop} isLoading />
            );
            const icons = document.querySelectorAll('[data-testid="icon"]');
            const hasSpinner = Array.from(icons).some((el) =>
              el.className.includes('animate-spin')
            );
            unmount();
            // animate-spin should be present iff motion is allowed
            return prefersReduced ? !hasSpinner : hasSpinner;
          }
        ),
        { numRuns: 100 }
      );
    }
  );

  it(
    // Feature: reduced-motion-swap-animations, Property 5: animate-pulse absent iff prefersReducedMotion is true
    'Property 5: animate-pulse absent on high-impact button iff prefersReducedMotion is true',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          const { unmount } = render(
            <SwapButton state="high_impact_warning" onSwap={noop} />
          );
          const btn = screen.getByRole('button');
          const hasPulse = btn.className.includes('animate-pulse');
          unmount();
          return prefersReduced ? !hasPulse : hasPulse;
        }),
        { numRuns: 100 }
      );
    }
  );
});
