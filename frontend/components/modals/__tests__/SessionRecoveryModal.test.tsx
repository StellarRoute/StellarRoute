import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SessionRecoveryModal } from '@/components/swap/SessionRecoveryModal';

describe('SessionRecoveryModal', () => {
  const mockOnRestore = vi.fn();
  const mockOnDismiss = vi.fn();

  beforeEach(() => {
    mockOnRestore.mockClear();
    mockOnDismiss.mockClear();
  });

  it('should not render when not open', () => {
    render(
      <SessionRecoveryModal
        isOpen={false}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('should show wake recovery title and description', () => {
    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="wake"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.getByText('Resume In-Progress Trade?')).toBeInTheDocument();
    expect(
      screen.getByText(/This tab was idle long enough/)
    ).toBeInTheDocument();
  });

  it('should show refresh recovery title and description', () => {
    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.getByText('Restore Previous Trade?')).toBeInTheDocument();
    expect(screen.getByText(/A saved draft was found/)).toBeInTheDocument();
  });

  it('should display snapshot summary when provided', () => {
    const snapshot = {
      amount: '100',
      slippage: 1.5,
      deadline: 45,
      fromToken: 'native',
      toToken: 'USDC:GQUOTE',
      savedAt: Date.now(),
    };

    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={snapshot}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const summary = screen.getByTestId('session-recovery-summary');
    expect(summary).toHaveTextContent('XLM to USDC');
    expect(summary).toHaveTextContent('100');
    expect(summary).toHaveTextContent('1.5%');
    expect(summary).toHaveTextContent('45 min');
  });

  it('should call onRestore when restore button clicked', async () => {
    mockOnRestore.mockResolvedValue(undefined);

    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const restoreButton = screen.getAllByTestId('restore-session-button')[0];
    fireEvent.click(restoreButton);

    await waitFor(() => {
      expect(mockOnRestore).toHaveBeenCalled();
    });
  });

  it('should call onDiscard when start fresh button clicked', () => {
    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const discardButton = screen.getAllByTestId('start-fresh-button')[0];
    fireEvent.click(discardButton);

    expect(mockOnDismiss).toHaveBeenCalled();
  });

  it('should show loading state during recovery', () => {
    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        isRecovering={true}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.getAllByText('Restoring...')[0]).toBeInTheDocument();
    const restoreButton = screen.getAllByText('Restoring...')[0];
    expect(restoreButton).toBeDisabled();
  });

  it('should display error message on restore failure', async () => {
    const errorMessage = 'Failed to refresh quote';
    mockOnRestore.mockRejectedValue(new Error(errorMessage));

    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const restoreButton = screen.getAllByTestId('restore-session-button')[0];
    fireEvent.click(restoreButton);

    await waitFor(() => {
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
    });
  });

  it('should disable buttons during recovery', () => {
    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        isRecovering={true}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const discardButton = screen.getAllByTestId('start-fresh-button')[0];
    expect(discardButton).toBeDisabled();
  });

  it('should show different action text for wake vs refresh', () => {
    const { rerender } = render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.getAllByText('Restore Session')[0]).toBeInTheDocument();

    rerender(
      <SessionRecoveryModal
        isOpen={true}
        reason="wake"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    expect(screen.getByText('Refresh Quote')).toBeInTheDocument();
  });

  it('should handle async restore with proper loading states', async () => {
    let resolveRestore: () => void;
    const restorePromise = new Promise<void>((resolve) => {
      resolveRestore = resolve;
    });
    mockOnRestore.mockReturnValue(restorePromise);

    render(
      <SessionRecoveryModal
        isOpen={true}
        reason="refresh"
        snapshot={null}
        onRestore={mockOnRestore}
        onDiscard={mockOnDismiss}
      />
    );

    const restoreButton = screen.getAllByTestId('restore-session-button')[0];
    fireEvent.click(restoreButton);

    // Should show loading state
    await waitFor(() => {
      expect(screen.getAllByText('Restoring...')[0]).toBeInTheDocument();
    });

    // Resolve the promise
    resolveRestore!();
    
    await waitFor(() => {
      expect(screen.queryByText('Restoring...')).not.toBeInTheDocument();
    });
  });
});
