import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { BatchSwapPreview } from './BatchSwapPreview';

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

// Mock useFeatureFlag to control the locked/unlocked state
vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: vi.fn(() => false), // default: feature disabled → locked state
}));

import { useFeatureFlag } from '@/hooks/useFeatureFlag';

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

describe('BatchSwapPreview — reduced-motion (locked state)', () => {
  afterEach(() => {
    setReducedMotion(false);
    vi.mocked(useFeatureFlag).mockReturnValue(false);
  });

  it('locked card does NOT have animate-in or fade-in when reduced motion is active', () => {
    setReducedMotion(true);
    vi.mocked(useFeatureFlag).mockReturnValue(false);
    render(<BatchSwapPreview legs={[]} />);
    // The locked card is the only Card rendered
    const card = document.querySelector('[data-slot="card"]') as HTMLElement;
    expect(card).toBeTruthy();
    expect(card.className).not.toContain('animate-in');
    expect(card.className).not.toContain('fade-in');
  });

  it('locked card HAS animate-in and fade-in when motion is allowed', () => {
    setReducedMotion(false);
    vi.mocked(useFeatureFlag).mockReturnValue(false);
    render(<BatchSwapPreview legs={[]} />);
    const card = document.querySelector('[data-slot="card"]') as HTMLElement;
    expect(card).toBeTruthy();
    expect(card.className).toContain('animate-in');
    expect(card.className).toContain('fade-in');
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('BatchSwapPreview — property tests', () => {
  afterEach(() => {
    setReducedMotion(false);
    vi.mocked(useFeatureFlag).mockReturnValue(false);
  });

  it(
    // Feature: reduced-motion-swap-animations, Property 11 & 12
    'Property 11 & 12: animate-in absent iff prefersReducedMotion is true on locked card',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (prefersReduced) => {
          setReducedMotion(prefersReduced);
          vi.mocked(useFeatureFlag).mockReturnValue(false);
          const { unmount } = render(<BatchSwapPreview legs={[]} />);
          const card = document.querySelector('[data-slot="card"]') as HTMLElement;
          const hasAnimateIn = card?.className.includes('animate-in') ?? false;
          unmount();

          return prefersReduced ? !hasAnimateIn : hasAnimateIn;
        }),
        { numRuns: 100 }
      );
    }
  );
});
