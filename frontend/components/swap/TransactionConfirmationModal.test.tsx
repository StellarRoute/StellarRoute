import { describe, it, expect, vi, afterEach } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import fc from 'fast-check';
import { TransactionConfirmationModal } from './TransactionConfirmationModal';
import { createSwapTranslator } from '@/lib/swap-i18n';

afterEach(() => {
  cleanup();
});

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
    expect(spinner.classList.contains('animate-spin')).toBe(false);
  });

  it('spinner HAS animate-spin when status=pending and motion is allowed', () => {
    setReducedMotion(false);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="pending" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.classList.contains('animate-spin')).toBe(true);
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
    expect(spinner.classList.contains('animate-spin')).toBe(false);
  });

  it('spinner HAS animate-spin when status=submitted and motion is allowed', () => {
    setReducedMotion(false);
    render(<TransactionConfirmationModal {...BASE_PROPS} status="submitted" />);
    const spinner = screen.getByTestId('tcm-spinner');
    expect(spinner.classList.contains('animate-spin')).toBe(true);
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
            cleanup();
            setReducedMotion(prefersReduced);
            const { unmount } = render(
              <TransactionConfirmationModal {...BASE_PROPS} status={status} />
            );
            const spinner = screen.getByTestId('tcm-spinner');
            const isPresent = !!spinner;
            const hasSpin = spinner.classList.contains('animate-spin');
            unmount();

            if (prefersReduced) {
              return isPresent && !hasSpin;
            } else {
              return isPresent && hasSpin;
            }
          }
        ),
        { numRuns: 10 }
      );
    }
  );
});

// ---------------------------------------------------------------------------
// i18n tests
// ---------------------------------------------------------------------------

// Mock useSwapI18n: returns a translator that passes keys back as-is (key-echo)
// so we can assert the component is calling t() with the right keys rather than
// rendering raw English strings.
vi.mock('@/lib/swap-i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/swap-i18n')>();
  return {
    ...actual,
    useSwapI18n: vi.fn(() => ({
      t: (key: string) => `[${key}]`,
    })),
  };
});

const TRADE_PARAMS = {
  fromAmount: '10',
  fromAsset: 'XLM',
  toAmount: '5',
  toAsset: 'USDC',
  minReceived: '4.975 USDC',
};

const I18N_BASE_PROPS = {
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

describe('TransactionConfirmationModal — i18n key usage', () => {
  afterEach(() => cleanup());

  it('renders the review heading via i18n key swap.confirm.review.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="review" />);
    expect(screen.getByText('[swap.confirm.review.heading]')).toBeInTheDocument();
  });

  it('renders the pending heading via i18n key swap.confirm.pending.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="pending" />);
    expect(screen.getByText('[swap.confirm.pending.heading]')).toBeInTheDocument();
  });

  it('renders the submitted heading via i18n key swap.confirm.submitted.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="submitted" />);
    expect(screen.getByText('[swap.confirm.submitted.heading]')).toBeInTheDocument();
  });

  it('renders the confirmed heading via i18n key swap.confirm.confirmed.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="confirmed" />);
    expect(screen.getByText('[swap.confirm.confirmed.heading]')).toBeInTheDocument();
  });

  it('renders the failed heading via i18n key swap.confirm.failed.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="failed" />);
    expect(screen.getByText('[swap.confirm.failed.heading]')).toBeInTheDocument();
  });

  it('renders the dropped heading via i18n key swap.confirm.dropped.heading', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="dropped" />);
    expect(screen.getByText('[swap.confirm.dropped.heading]')).toBeInTheDocument();
  });

  it('renders trade-summary labels via i18n keys when tradeParams supplied in review state', () => {
    render(
      <TransactionConfirmationModal
        {...I18N_BASE_PROPS}
        status="review"
        tradeParams={TRADE_PARAMS}
      />
    );
    expect(screen.getByText('[swap.confirm.summary.youPay]')).toBeInTheDocument();
    expect(screen.getByText('[swap.confirm.summary.youReceive]')).toBeInTheDocument();
    expect(screen.getByText('[swap.confirm.summary.minReceived]')).toBeInTheDocument();
  });

  it('does NOT render hardcoded English "You pay" in trade summary', () => {
    render(
      <TransactionConfirmationModal
        {...I18N_BASE_PROPS}
        status="review"
        tradeParams={TRADE_PARAMS}
      />
    );
    expect(screen.queryByText('You pay')).not.toBeInTheDocument();
    expect(screen.queryByText('You receive')).not.toBeInTheDocument();
    expect(screen.queryByText('Min received')).not.toBeInTheDocument();
  });

  it('renders Confirm Swap CTA via i18n key swap.confirm.cta.confirmSwap', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="review" />);
    expect(screen.getByRole('button', { name: '[swap.confirm.cta.confirmSwap]' })).toBeInTheDocument();
  });

  it('renders Cancel CTA via i18n key swap.confirm.cta.cancel in review state', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="review" />);
    expect(screen.getByRole('button', { name: '[swap.confirm.cta.cancel]' })).toBeInTheDocument();
  });

  it('renders Done CTA via i18n key swap.confirm.cta.done in confirmed state', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="confirmed" />);
    expect(screen.getByRole('button', { name: '[swap.confirm.cta.done]' })).toBeInTheDocument();
  });

  it('renders Try Again CTA via i18n key swap.confirm.cta.tryAgain in failed state', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="failed" />);
    expect(screen.getByRole('button', { name: '[swap.confirm.cta.tryAgain]' })).toBeInTheDocument();
  });

  it('renders Dismiss CTA via i18n key swap.confirm.cta.dismiss in failed state', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="failed" />);
    expect(screen.getAllByRole('button', { name: '[swap.confirm.cta.dismiss]' })[0]).toBeInTheDocument();
  });

  it('renders Resubmit CTA via i18n key swap.confirm.cta.resubmit in dropped state', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="dropped" />);
    expect(screen.getByRole('button', { name: '[swap.confirm.cta.resubmit]' })).toBeInTheDocument();
  });

  it('does NOT render hardcoded English CTA labels', () => {
    render(<TransactionConfirmationModal {...I18N_BASE_PROPS} status="review" />);
    expect(screen.queryByRole('button', { name: 'Confirm Swap' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument();
  });

  it('zh-CN locale renders correct heading translation for review status', () => {
    const { t } = createSwapTranslator('zh-CN');
    expect(t('swap.confirm.review.heading')).toBe('检查兑换');
  });

  it('zh-CN locale renders correct heading translation for confirmed status', () => {
    const { t } = createSwapTranslator('zh-CN');
    expect(t('swap.confirm.confirmed.heading')).toBe('兑换已确认');
  });

  it('zh-CN locale renders correct CTA translation for confirmSwap', () => {
    const { t } = createSwapTranslator('zh-CN');
    expect(t('swap.confirm.cta.confirmSwap')).toBe('确认兑换');
  });

  it('en-US locale renders correct summary label translations', () => {
    const { t } = createSwapTranslator('en-US');
    expect(t('swap.confirm.summary.youPay')).toBe('You pay');
    expect(t('swap.confirm.summary.youReceive')).toBe('You receive');
    expect(t('swap.confirm.summary.minReceived')).toBe('Min received');
  });
});
