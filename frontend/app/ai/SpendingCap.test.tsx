import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { IntentPreviewCard } from './IntentPreviewCard';
import { AgentChat } from './AgentChat';
import { loadCap } from '@/lib/ai/cap';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn() }),
}));

const SEND = { kind: 'send' as const, amount: '150', fromAsset: 'USDC', recipient: 'GABC' };

describe('per-confirm USDC cap on /ai', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('amount above cap disables Confirm and explains the cap', () => {
    const onConfirm = vi.fn();
    render(<IntentPreviewCard intent={SEND} usdcCap={100} onConfirm={onConfirm} />);

    const confirm = screen.getByTestId('confirm-btn');
    expect(confirm).toBeDisabled();
    expect(screen.getByTestId('cap-exceeded')).toHaveTextContent(
      'Above your 100 USDC per-confirm cap',
    );
    fireEvent.click(confirm);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('amount within cap still confirms locally', () => {
    const onConfirm = vi.fn();
    render(<IntentPreviewCard intent={SEND} usdcCap={200} onConfirm={onConfirm} />);

    expect(screen.queryByTestId('cap-exceeded')).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId('confirm-btn'));
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it('unset cap adds no block', () => {
    render(<IntentPreviewCard intent={{ ...SEND, amount: '1000000' }} />);
    expect(screen.getByTestId('confirm-btn')).not.toBeDisabled();
  });

  it('settings section saves the cap locally and clears it', async () => {
    render(<AgentChat />);

    fireEvent.change(screen.getByTestId('cap-input'), { target: { value: '25' } });
    fireEvent.click(screen.getByTestId('cap-save'));

    await waitFor(() => expect(loadCap()).toBe(25));
    expect(screen.getByTestId('ai-settings')).toHaveTextContent('disabled above 25 USDC');

    fireEvent.click(screen.getByTestId('cap-clear'));
    await waitFor(() => expect(loadCap()).toBeNull());
  });
});
