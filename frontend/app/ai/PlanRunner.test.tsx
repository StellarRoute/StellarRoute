import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PlanRunner } from './PlanRunner';
import { loadTranscript } from '@/lib/ai/transcript';
import type { Intent } from '@/lib/ai/parse';

const mockPush = vi.hoisted(() => vi.fn());

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: mockPush, replace: vi.fn(), prefetch: vi.fn() }),
}));

const STEPS: Intent[] = [
  { type: 'bridge', amount: '20', asset: 'USDC', destination: 'sepolia' },
  { type: 'cash_out', amount: '20', asset: 'USDC' },
];

describe('PlanRunner', () => {
  beforeEach(() => {
    mockPush.mockClear();
    localStorage.clear();
  });

  it('shows both step labels with Confirm only on step 1', () => {
    render(<PlanRunner steps={STEPS} />);

    expect(screen.getByTestId('plan-step-1-label')).toHaveTextContent('Bridge 20 USDC to sepolia');
    expect(screen.getByTestId('plan-step-2-label')).toHaveTextContent('Cash out 20 USDC');
    expect(screen.getByTestId('plan-step-1-confirm')).toBeInTheDocument();
    expect(screen.queryByTestId('plan-step-2-confirm')).not.toBeInTheDocument();
    expect(screen.getByTestId('plan-step-2')).toHaveAttribute('data-state', 'pending');
  });

  it('handing off step 1 marks it done in the transcript and activates step 2', () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    render(<PlanRunner steps={STEPS} />);

    fireEvent.click(screen.getByTestId('plan-step-1-confirm'));

    expect(mockPush).toHaveBeenCalledWith('/cross-chain-swap');
    expect(screen.getByTestId('plan-step-1')).toHaveAttribute('data-state', 'done');
    expect(screen.queryByTestId('plan-step-1-confirm')).not.toBeInTheDocument();
    expect(screen.getByTestId('plan-step-2-confirm')).toBeInTheDocument();
    expect(loadTranscript().map((m) => m.content)).toEqual([
      'Step 1 done: Bridge 20 USDC to sepolia',
    ]);

    fireEvent.click(screen.getByTestId('plan-step-2-confirm'));
    expect(mockPush).toHaveBeenLastCalledWith('/offramp?amount=20&source=stellar-usdc');

    // Handoff is navigation only: no wallet, prepare, or submit calls.
    expect(fetchSpy).not.toHaveBeenCalled();
    fetchSpy.mockRestore();
  });

  it('disables the current step Confirm when above the USDC cap', () => {
    render(<PlanRunner steps={STEPS} usdcCap={10} />);

    expect(screen.getByTestId('plan-step-1-confirm')).toBeDisabled();
    expect(screen.getByTestId('cap-exceeded')).toHaveTextContent('10 USDC per-confirm cap');
  });
});
