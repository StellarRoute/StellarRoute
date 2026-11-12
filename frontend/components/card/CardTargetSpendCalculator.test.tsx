import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardTargetSpendCalculator } from './CardTargetSpendCalculator';

describe('CardTargetSpendCalculator', () => {
  it('shows 100.00 USDC for 90 EUR with 0 balance', () => {
    render(<CardTargetSpendCalculator currentUsdc={0} />);
    fireEvent.change(screen.getByTestId('fiat-input'), {
      target: { value: '90' },
    });
    expect(screen.getByTestId('required-usdc').textContent).toContain('100.00');
  });

  it('shows indicative badge', () => {
    render(<CardTargetSpendCalculator />);
    expect(screen.getByTestId('indicative-badge').textContent).toMatch(
      /indicative/i,
    );
  });

  it('calls onBridgeRequired with deficit', () => {
    const onBridge = vi.fn();
    render(
      <CardTargetSpendCalculator currentUsdc={20} onBridgeRequired={onBridge} />,
    );
    fireEvent.change(screen.getByTestId('fiat-input'), {
      target: { value: '90' },
    });
    fireEvent.click(screen.getByTestId('bridge-required-btn'));
    expect(onBridge).toHaveBeenCalledWith(80);
  });
});