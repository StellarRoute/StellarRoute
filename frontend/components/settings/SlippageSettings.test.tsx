import { useState } from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { SlippageSettings } from './SlippageSettings';

const LAST_PRESET_STORAGE_KEY = 'stellar_route_last_slippage_preset';

function SlippageHarness({ initial = 0.5 }: { initial?: number }) {
  const [value, setValue] = useState(initial);
  return <SlippageSettings value={value} onChange={setValue} />;
}

describe('SlippageSettings', () => {
  beforeEach(() => {
    window.localStorage.removeItem(LAST_PRESET_STORAGE_KEY);
  });

  afterEach(cleanup);

  it('renders the controlled default value', () => {
    render(<SlippageHarness />);
    expect(screen.getByText('Slippage Tolerance')).toBeInTheDocument();
    expect(screen.getByText('0.5%')).toBeInTheDocument();
  });

  it('changes value when a preset button is clicked', async () => {
    const user = userEvent.setup();
    render(<SlippageHarness />);

    await user.click(screen.getByRole('button', { name: 'Aggressive' }));

    expect(screen.getByText('1%')).toBeInTheDocument();
  });

  it('handles custom values and risk warnings', async () => {
    const user = userEvent.setup();
    render(<SlippageHarness />);
    const input = screen.getByPlaceholderText('Custom');

    await user.type(input, '0.05');
    expect(
      screen.getByText(/may fail if the price moves/i)
    ).toBeInTheDocument();

    await user.clear(input);
    await user.type(input, '10');
    expect(
      screen.getByText(/increases the risk of frontrunning/i)
    ).toBeInTheDocument();
  });

  it('remembers the last preset chosen and persists it to localStorage', async () => {
    const user = userEvent.setup();
    render(<SlippageHarness />);

    await user.click(screen.getByRole('button', { name: 'Aggressive' }));

    expect(window.localStorage.getItem(LAST_PRESET_STORAGE_KEY)).toBe('1');
  });

  it('applies the remembered preset on mount when it differs from the default', () => {
    window.localStorage.setItem(LAST_PRESET_STORAGE_KEY, '1');

    render(<SlippageHarness initial={0.5} />);

    expect(screen.getByText('1%')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Aggressive' })).toHaveAttribute(
      'data-variant',
      'default'
    );
  });

  it('ignores a corrupted stored preset value', () => {
    window.localStorage.setItem(LAST_PRESET_STORAGE_KEY, 'not-a-number');

    render(<SlippageHarness initial={0.5} />);

    expect(screen.getByText('0.5%')).toBeInTheDocument();
  });
});
