import { useState } from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { SettingsPanel } from './SettingsPanel';

function SettingsHarness() {
  const [slippage, setSlippage] = useState(0.5);
  const [deadline, setDeadline] = useState(30);
  const [expertMode, setExpertMode] = useState(false);
  const [bypassConfirmation, setBypassConfirmation] = useState(false);

  return (
    <SettingsPanel
      slippage={slippage}
      deadline={deadline}
      expertMode={expertMode}
      bypassConfirmation={bypassConfirmation}
      extendedRouteDetails={false}
      onSlippageChange={setSlippage}
      onDeadlineChange={setDeadline}
      onExpertModeChange={setExpertMode}
      onBypassConfirmationChange={setBypassConfirmation}
      onExtendedRouteDetailsChange={vi.fn()}
      onReset={() => {
        setSlippage(0.5);
        setDeadline(30);
        setExpertMode(false);
        setBypassConfirmation(false);
      }}
    />
  );
}

describe('SettingsPanel', () => {
  afterEach(cleanup);

  it('opens with the live Advanced Settings title', async () => {
    const user = userEvent.setup();
    render(<SettingsHarness />);

    await user.click(screen.getByRole('button', { name: /settings/i }));

    expect(
      screen.getByRole('heading', { name: 'Advanced Settings' })
    ).toBeInTheDocument();
    expect(screen.getByText('Slippage Tolerance')).toBeInTheDocument();
    expect(screen.getByText('Transaction Deadline')).toBeInTheDocument();
  });

  it('updates controlled trade parameters and resets them', async () => {
    const user = userEvent.setup();
    render(<SettingsHarness />);
    await user.click(screen.getByRole('button', { name: /settings/i }));

    await user.click(screen.getByRole('button', { name: /aggressive/i }));
    await user.click(screen.getByRole('button', { name: /^1h$/i }));
    expect(screen.getByText('1%')).toBeInTheDocument();
    expect(screen.getByText('60 min')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /reset/i }));
    expect(screen.getByText('0.5%')).toBeInTheDocument();
    expect(screen.getByText('30 min')).toBeInTheDocument();
  });

  it('exposes expert confirmation bypass controls', async () => {
    const user = userEvent.setup();
    render(<SettingsHarness />);
    await user.click(screen.getByRole('button', { name: /settings/i }));
    await user.click(screen.getByRole('switch', { name: /expert mode/i }));

    expect(screen.getByText('Bypass Confirmation Modal')).toBeInTheDocument();
  });
});
