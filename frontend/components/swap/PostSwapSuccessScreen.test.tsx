import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PostSwapSuccessScreen } from './PostSwapSuccessScreen';

// ── Mocks ───────────────────────────────────────────────────────────────────

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Setup minimal ResizeObserver mock if needed by radix ui or lucide
if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

describe('PostSwapSuccessScreen', () => {
  const defaultProps = {
    txHash: '0x123abc456def',
    onDone: vi.fn(),
    onSwapAgain: vi.fn(),
  };

  const mockClipboardWriteText = vi.fn().mockResolvedValue(undefined);
  const mockShare = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: { writeText: mockClipboardWriteText },
      share: mockShare,
      canShare: () => true,
    });
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders swap confirmed state and hash correctly', () => {
    render(<PostSwapSuccessScreen {...defaultProps} />);
    
    expect(screen.getByText('Swap confirmed')).toBeInTheDocument();
    expect(screen.getByText('Your swap is complete')).toBeInTheDocument();
    
    expect(screen.getByText('0x123abc456def')).toBeInTheDocument();
  });

  it('renders trade parameters when provided', () => {
    render(
      <PostSwapSuccessScreen 
        {...defaultProps}
        tradeParams={{
          fromAmount: '100',
          fromAsset: 'XLM',
          toAmount: '50',
          toAsset: 'USDC',
          networkFee: '0.01 XLM',
        }}
      />
    );
    
    expect(screen.getByText('100 XLM')).toBeInTheDocument();
    expect(screen.getByText('50 USDC')).toBeInTheDocument();
    expect(screen.getByText('0.01 XLM')).toBeInTheDocument();
  });

  it('calls onDone when Done is clicked (dismiss)', async () => {
    const user = userEvent.setup();
    render(<PostSwapSuccessScreen {...defaultProps} />);
    
    await user.click(screen.getByRole('button', { name: /done/i }));
    expect(defaultProps.onDone).toHaveBeenCalledOnce();
  });

  it('calls onSwapAgain when Swap Again is clicked', async () => {
    const user = userEvent.setup();
    render(<PostSwapSuccessScreen {...defaultProps} />);
    
    await user.click(screen.getByRole('button', { name: /swap again/i }));
    expect(defaultProps.onSwapAgain).toHaveBeenCalledOnce();
  });

  it('includes screen reader labels for transaction status actions', () => {
    render(<PostSwapSuccessScreen {...defaultProps} />);
    
    // Verify Share has an aria-label
    expect(screen.getByLabelText('Share explorer link')).toBeInTheDocument();
    
    // Verify CopyButton has received the label
    expect(screen.getByLabelText('Copy transaction hash')).toBeInTheDocument();
  });

  it('shares using Web Share API when supported', async () => {
    const user = userEvent.setup();
    render(<PostSwapSuccessScreen {...defaultProps} />);
    
    await user.click(screen.getByLabelText('Share explorer link'));
    
    expect(mockShare).toHaveBeenCalledOnce();
    expect(mockShare).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'StellarRoute Swap',
        url: expect.stringContaining('0x123abc456def'),
      })
    );
  });

  it('falls back to clipboard copy if Web Share API is not supported', async () => {
    // Override canShare to simulate unsupported environment
    Object.assign(navigator, { canShare: undefined, share: undefined });
    const user = userEvent.setup();
    
    render(<PostSwapSuccessScreen {...defaultProps} />);
    await user.click(screen.getByLabelText('Share explorer link'));
    
    expect(mockClipboardWriteText).toHaveBeenCalledOnce();
    expect(mockClipboardWriteText).toHaveBeenCalledWith(
      expect.stringContaining('0x123abc456def')
    );
    expect(screen.getByText(/copied/i)).toBeInTheDocument();
  });
});
