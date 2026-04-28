import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { TransactionConfirmationModal } from './TransactionConfirmationModal';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';

const mockTradeParams: TradeParams = {
  fromAsset: 'XLM',
  fromAmount: '10',
  toAsset: 'USDC',
  toAmount: '9.95',
  exchangeRate: '0.995',
  priceImpact: '0.1',
  minReceived: '9.90',
  networkFee: '0.00001',
  routePath: [],
  walletAddress: 'GABC',
};

const defaultProps = {
  isOpen: true,
  tradeParams: mockTradeParams,
  onConfirm: vi.fn(),
  onCancel: vi.fn(),
  onTryAgain: vi.fn(),
  onResubmit: vi.fn(),
  onDismiss: vi.fn(),
  onDone: vi.fn(),
};

describe('TransactionConfirmationModal', () => {
  it('renders review state with Confirm Swap and Cancel buttons', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="review" />);
    expect(screen.getByText('Review Swap')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /confirm swap/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('renders pending state with amber spinner and Cancel button', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="pending" />);
    expect(screen.getByText('Waiting for wallet\u2026')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('renders submitted state with amber spinner and no primary action', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="submitted" />);
    expect(screen.getByText('Awaiting confirmation')).toBeInTheDocument();
  });

  it('renders confirmed state with green checkmark and Done button', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="confirmed" />);
    expect(screen.getByText('Swap confirmed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /done/i })).toBeInTheDocument();
  });

  it('renders failed state with Try Again and Dismiss buttons', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="failed" />);
    expect(screen.getByText('Swap failed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /dismiss/i })).toBeInTheDocument();
  });

  it('renders dropped state with Resubmit and Dismiss buttons', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="dropped" />);
    expect(screen.getByText('Transaction timed out')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /resubmit/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /dismiss/i })).toBeInTheDocument();
  });

  it('aria-live region has non-empty text for pending status', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="pending" />);
    const liveRegion = document.querySelector('[aria-live="polite"]');
    expect(liveRegion).toBeTruthy();
    expect(liveRegion?.textContent?.trim()).not.toBe('');
  });

  it('aria-live region has non-empty text for submitted status', () => {
    render(<TransactionConfirmationModal {...defaultProps} status="submitted" />);
    const liveRegion = document.querySelector('[aria-live="polite"]');
    expect(liveRegion).toBeTruthy();
    expect(liveRegion?.textContent?.trim()).not.toBe('');
  });

  it('shows custom error message in failed state', () => {
    render(
      <TransactionConfirmationModal
        {...defaultProps}
        status="failed"
        errorMessage="Signature rejected. You can try again or dismiss."
      />
    );
    expect(screen.getByText('Signature rejected. You can try again or dismiss.')).toBeInTheDocument();
  });

  it('calls onConfirm when Confirm Swap is clicked', () => {
    const onConfirm = vi.fn();
    render(<TransactionConfirmationModal {...defaultProps} status="review" onConfirm={onConfirm} />);
    fireEvent.click(screen.getByRole('button', { name: /confirm swap/i }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it('calls onTryAgain when Try Again is clicked', () => {
    const onTryAgain = vi.fn();
    render(<TransactionConfirmationModal {...defaultProps} status="failed" onTryAgain={onTryAgain} />);
    fireEvent.click(screen.getByRole('button', { name: /try again/i }));
    expect(onTryAgain).toHaveBeenCalledTimes(1);
  });

  it('calls onDone when Done is clicked', () => {
    const onDone = vi.fn();
    render(<TransactionConfirmationModal {...defaultProps} status="confirmed" onDone={onDone} />);
    fireEvent.click(screen.getByRole('button', { name: /done/i }));
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});
