import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { TransactionConfirmationModal } from './TransactionConfirmationModal';

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

const BASE_PROPS = {
  isOpen: true,
  txHash: undefined,
  errorMessage: undefined,
  tradeParams: undefined,
  onConfirm: vi.fn(),
  onCancel: vi.fn(),
  onTryAgain: vi.fn(),
  onResubmit: vi.fn(),
  onDismiss: vi.fn(),
  onDone: vi.fn(),
};

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

describe('TransactionConfirmationModal — reduced-motion', () => {
  afterEach(() => setReducedMotion(false));

  it('spinner is present in the DOM when status=pending and reduced motion is active', () => {
    setReducedMotion(true);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="pending" />);
    expect(screen.getByTestId('tcm-spinner')).toBeInTheDocument();
  });

  it('spinner does NOT have animate-spin when status=pending and reduced motion is active', () => {
    setReducedMotion(true);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="pending" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.className).not.toContain('animate-spin');
  });

  it('spinner HAS animate-spin when status=pending and motion is allowed', () => {
    setReducedMotion(false);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="pending" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.className).toContain('animate-spin');
  });

  it('spinner is present in the DOM when status=submitted and reduced motion is active', () => {
    setReducedMotion(true);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="submitted" />);
    expect(screen.getByTestId('tcm-spinner')).toBeInTheDocument();
  });

  it('spinner does NOT have animate-spin when status=submitted and reduced motion is active', () => {
    setReducedMotion(true);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="submitted" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.className).not.toContain('animate-spin');
  });

  it('spinner HAS animate-spin when status=submitted and motion is allowed', () => {
    setReducedMotion(false);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="submitted" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.className).toContain('animate-spin');
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('TransactionConfirmationModal — property tests', () => {
  afterEach(() => setReducedMotion(false));

  it(
    // Feature: reduced-motion-swap-animations, Property 9 & 10
    'Property 9 & 10: animate-spin absent iff prefersReducedMotion is true; spinner always present',
    () => {
      fc.assert(
        fc.property(
          fc.boolean(),
          fc.constantFrom('pending' as const, 'submitted' as const),
          (prefersReduced, status) => {
            setReducedMotion(prefersReduced);
            const { unmount } = render(
              <TransactionConfirmationModal {...BASE_PROPS} status={status} />
            );
            const spinner = screen.getByTestId('tcm-spinner');
            const isPresent = !!spinner;
            const hasSpin = spinner.className.includes('animate-spin');
            unmount();

            if (prefersReduced) {
              return isPresent && !hasSpin;
            } else {
              return isPresent && hasSpin;
            }
          }
        ),
        { numRuns: 100 }
      );
    }
  );
});
