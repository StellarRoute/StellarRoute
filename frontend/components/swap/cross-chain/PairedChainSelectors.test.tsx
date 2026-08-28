import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PairedChainSelectors } from './PairedChainSelectors';

describe('PairedChainSelectors', () => {
  it('uses full-label radio hit targets that accept direct clicks', async () => {
    const user = userEvent.setup();
    const onDestChange = vi.fn();

    render(
      <PairedChainSelectors
        sourceChainId="stellar"
        destChainId="stellar"
        onSourceChange={vi.fn()}
        onDestChange={onDestChange}
      />,
    );

    const option = screen.getByTestId('chain-option-destination-ethereum-sepolia');
    const label = option.closest('label');

    expect(label).not.toBeNull();
    expect(label?.className).toMatch(/min-h-11/);
    expect(option.className).toMatch(/absolute/);
    expect(option.className).not.toMatch(/sr-only/);

    await user.click(option);
    expect(onDestChange).toHaveBeenCalledWith('ethereum-sepolia');
  });

  it('supports keyboard selection via native radio semantics', async () => {
    const user = userEvent.setup();
    const onDestChange = vi.fn();

    render(
      <PairedChainSelectors
        sourceChainId="stellar"
        destChainId="stellar"
        onSourceChange={vi.fn()}
        onDestChange={onDestChange}
      />,
    );

    const solana = screen.getByTestId('chain-option-destination-solana');
    solana.focus();
    await user.keyboard(' ');

    expect(onDestChange).toHaveBeenCalledWith('solana');
  });
});
