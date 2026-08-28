import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { OfframpModeToggle } from './OfframpModeToggle';

describe('OfframpModeToggle', () => {
  it('renders a radiogroup labelled as the offramp path', () => {
    render(<OfframpModeToggle mode="direct" onChange={() => {}} />);

    expect(screen.getByTestId('offramp-mode-toggle')).toBeInTheDocument();
    expect(
      screen.getByRole('radiogroup', { name: /offramp path/i }),
    ).toBeInTheDocument();
  });

  it('renders both selectable modes', () => {
    render(<OfframpModeToggle mode="direct" onChange={() => {}} />);

    expect(screen.getByTestId('offramp-mode-direct')).toBeInTheDocument();
    expect(screen.getByTestId('offramp-mode-bridge')).toBeInTheDocument();

    expect(screen.getByText(/Stellar USDC/i)).toBeInTheDocument();
    expect(screen.getByText(/Bridge \+ offramp/i)).toBeInTheDocument();
  });

  it('marks the current mode as selected', () => {
    render(<OfframpModeToggle mode="direct" onChange={() => {}} />);

    expect(screen.getByTestId('offramp-mode-direct')).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByTestId('offramp-mode-bridge')).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  it('marks bridge as selected when in bridge mode', () => {
    render(<OfframpModeToggle mode="bridge" onChange={() => {}} />);

    expect(screen.getByTestId('offramp-mode-bridge')).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByTestId('offramp-mode-direct')).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  it('invokes onChange with the direct mode when toggled', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<OfframpModeToggle mode="bridge" onChange={onChange} />);

    await user.click(screen.getByTestId('offramp-mode-direct'));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('direct');
  });

  it('invokes onChange with the bridge mode when toggled', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<OfframpModeToggle mode="direct" onChange={onChange} />);

    await user.click(screen.getByTestId('offramp-mode-bridge'));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('bridge');
  });
});
