import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { RouteDisclosurePanel } from './RouteDisclosurePanel';

describe('RouteDisclosurePanel', () => {
  it('uses semantic details disclosure with keyboard-usable summary', () => {
    render(<RouteDisclosurePanel />);
    const panel = screen.getByTestId('route-disclosure-panel');
    expect(panel.tagName).toBe('DETAILS');
    expect(screen.getByText('Before you route')).toBeInTheDocument();
    expect(screen.getByText('Show details')).toBeInTheDocument();
    expect(screen.getByText(/non-custodial/i)).not.toBeVisible();
  });

  it('reveals disclosure copy when opened', async () => {
    const user = userEvent.setup();
    render(<RouteDisclosurePanel />);
    await user.click(screen.getByText('Before you route'));
    expect(screen.getByText(/non-custodial/i)).toBeVisible();
    expect(screen.getByText('Hide details')).toBeInTheDocument();
  });
});
