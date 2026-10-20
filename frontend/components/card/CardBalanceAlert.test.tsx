import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';

import {
  CardBalanceAlert,
  cardBalanceAlertDismissKey,
} from './CardBalanceAlert';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';

function dismissStorageKey(address = ADDRESS, threshold = 25) {
  return cardBalanceAlertDismissKey(address, threshold);
}

beforeEach(() => {
  window.sessionStorage.clear();
  window.localStorage.clear();
});

describe('CardBalanceAlert visibility', () => {
  it('renders when the available balance is strictly below the threshold (24.99)', () => {
    render(<CardBalanceAlert availableUsdc={24.99} />);

    expect(screen.getByTestId('card-balance-alert')).toBeInTheDocument();
  });

  it('renders at a zero balance', () => {
    render(<CardBalanceAlert availableUsdc={0} />);

    expect(screen.getByTestId('card-balance-alert')).toBeInTheDocument();
  });

  it('does not render exactly at the threshold (25.00)', () => {
    render(<CardBalanceAlert availableUsdc={25} />);

    expect(screen.queryByTestId('card-balance-alert')).not.toBeInTheDocument();
  });

  it('does not render above the threshold', () => {
    render(<CardBalanceAlert availableUsdc={25.01} />);

    expect(screen.queryByTestId('card-balance-alert')).not.toBeInTheDocument();
  });

  it('does not render while the balance is unknown', () => {
    render(<CardBalanceAlert availableUsdc={null} />);
    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();

    const { unmount } = render(<CardBalanceAlert availableUsdc={undefined} />);
    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();
    unmount();
  });

  it('honours a custom threshold from preferences', () => {
    const { rerender } = render(
      <CardBalanceAlert availableUsdc={30} thresholdUsdc={40} />,
    );
    expect(screen.getByTestId('card-balance-alert')).toBeInTheDocument();

    rerender(<CardBalanceAlert availableUsdc={40} thresholdUsdc={40} />);
    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();
  });

  it('shows the current, threshold, and recommended amounts', () => {
    render(
      <CardBalanceAlert
        availableUsdc={18.4}
        thresholdUsdc={25}
        recommendedUsdc={50}
      />,
    );

    const text = screen.getByTestId('card-balance-alert').textContent ?? '';
    expect(text).toContain('18.40 USDC available');
    expect(text).toContain('25.00 USDC alert threshold');
    expect(text).toContain('50.00 USDC');
  });
});

describe('CardBalanceAlert top-up trigger', () => {
  it('passes the recommended 50 USDC default to onTopUp', () => {
    const onTopUp = vi.fn();
    render(<CardBalanceAlert availableUsdc={10} onTopUp={onTopUp} />);

    fireEvent.click(screen.getByTestId('card-balance-alert-top-up'));

    expect(onTopUp).toHaveBeenCalledTimes(1);
    expect(onTopUp).toHaveBeenCalledWith(50);
  });

  it('passes a custom recommended amount to onTopUp', () => {
    const onTopUp = vi.fn();
    render(
      <CardBalanceAlert
        availableUsdc={10}
        recommendedUsdc={100}
        onTopUp={onTopUp}
      />,
    );

    fireEvent.click(screen.getByTestId('card-balance-alert-top-up'));

    expect(onTopUp).toHaveBeenCalledWith(100);
  });
});

describe('CardBalanceAlert dismissal', () => {
  it('hides the banner for the session and persists the flag under the dismiss key', () => {
    const onDismiss = vi.fn();
    render(
      <CardBalanceAlert
        availableUsdc={10}
        dismissKey={dismissStorageKey()}
        onDismiss={onDismiss}
      />,
    );

    fireEvent.click(screen.getByTestId('card-balance-alert-dismiss'));

    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();
    expect(onDismiss).toHaveBeenCalledTimes(1);
    expect(window.sessionStorage.getItem(dismissStorageKey())).toBe('true');
    expect(window.localStorage.getItem(dismissStorageKey())).toBeNull();
  });

  it('stays dismissed across remounts within the same session', () => {
    const { unmount } = render(
      <CardBalanceAlert availableUsdc={10} dismissKey={dismissStorageKey()} />,
    );
    fireEvent.click(screen.getByTestId('card-balance-alert-dismiss'));
    unmount();

    render(
      <CardBalanceAlert availableUsdc={10} dismissKey={dismissStorageKey()} />,
    );

    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();
  });

  it('alerts again for a different address or threshold scope', () => {
    render(
      <CardBalanceAlert availableUsdc={10} dismissKey={dismissStorageKey()} />,
    );
    fireEvent.click(screen.getByTestId('card-balance-alert-dismiss'));

    const otherAddressKey = cardBalanceAlertDismissKey(
      'GOTHEROTHEROTHEROTHEROTHEROTHEROTHEROTHEROTHEROTHEROOOO',
      25,
    );
    render(
      <CardBalanceAlert availableUsdc={10} dismissKey={otherAddressKey} />,
    );

    expect(screen.getAllByTestId('card-balance-alert')).toHaveLength(1);
  });

  it('re-reads dismissal when the dismiss key changes', async () => {
    const key = dismissStorageKey();
    window.sessionStorage.setItem(key, 'true');

    const { rerender } = render(
      <CardBalanceAlert availableUsdc={10} dismissKey="other-key" />,
    );
    expect(screen.getByTestId('card-balance-alert')).toBeInTheDocument();

    rerender(<CardBalanceAlert availableUsdc={10} dismissKey={key} />);

    await vi.waitFor(() =>
      expect(
        screen.queryByTestId('card-balance-alert'),
      ).not.toBeInTheDocument(),
    );
  });
});
