import { useState } from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { SlippageSettings } from './SlippageSettings';

function SlippageHarness() {
  const [value, setValue] = useState(0.5);
  return <SlippageSettings value={value} onChange={setValue} />;
}

describe('SlippageSettings', () => {
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
});
