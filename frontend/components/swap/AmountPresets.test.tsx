import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AmountPresets } from './AmountPresets';

describe('AmountPresets', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders all preset buttons', () => {
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance="100" onSelect={onSelect} />);

    const buttons = container.querySelectorAll('button');
    expect(buttons).toHaveLength(4);
    expect(buttons[0]).toHaveTextContent('25%');
    expect(buttons[1]).toHaveTextContent('50%');
    expect(buttons[2]).toHaveTextContent('75%');
    expect(buttons[3]).toHaveTextContent('100%');
  });

  it('calls onSelect with correct percentage when clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance="100" onSelect={onSelect} />);

    const buttons = container.querySelectorAll('button');
    await user.click(buttons[1]); // 50%
    expect(onSelect).toHaveBeenCalledWith(0.5);
  });

  it('disables buttons when balance is null', () => {
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance={null} onSelect={onSelect} />);

    const buttons = container.querySelectorAll('button');
    buttons.forEach((button) => {
      expect(button).toBeDisabled();
    });
  });

  it('disables buttons when balance is zero', () => {
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance="0" onSelect={onSelect} />);

    const buttons = container.querySelectorAll('button');
    buttons.forEach((button) => {
      expect(button).toBeDisabled();
    });
  });

  it('disables buttons when disabled prop is true', () => {
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance="100" onSelect={onSelect} disabled />);

    const buttons = container.querySelectorAll('button');
    buttons.forEach((button) => {
      expect(button).toBeDisabled();
    });
  });

  it('has proper accessibility labels', () => {
    const onSelect = vi.fn();
    const { container } = render(<AmountPresets balance="100" onSelect={onSelect} />);

    const buttons = container.querySelectorAll('button');
    expect(buttons[0]).toHaveAttribute('aria-label', 'Set amount to 25% of balance');
    expect(buttons[1]).toHaveAttribute('aria-label', 'Set amount to 50% of balance');
    expect(buttons[2]).toHaveAttribute('aria-label', 'Set amount to 75% of balance');
    expect(buttons[3]).toHaveAttribute('aria-label', 'Set amount to 100% of balance');
  });
});
