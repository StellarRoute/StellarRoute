import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { CardNetworkStatusBanner } from './CardNetworkStatusBanner';

function setOnline(value: boolean) {
  Object.defineProperty(window.navigator, 'onLine', {
    value,
    configurable: true,
  });
}

function goOffline() {
  act(() => {
    window.dispatchEvent(new Event('offline'));
  });
}

function goOnline() {
  act(() => {
    window.dispatchEvent(new Event('online'));
  });
}

afterEach(() => {
  setOnline(true);
});

describe('CardNetworkStatusBanner', () => {
  it('renders no banner when online', () => {
    render(
      <CardNetworkStatusBanner>
        <button type="button">Freeze Card</button>
      </CardNetworkStatusBanner>
    );
    expect(
      screen.queryByTestId('card-network-status-banner')
    ).not.toBeInTheDocument();
  });

  it('shows the amber banner immediately when the client goes offline', () => {
    render(
      <CardNetworkStatusBanner>
        <button type="button">Freeze Card</button>
      </CardNetworkStatusBanner>
    );
    goOffline();
    const banner = screen.getByTestId('card-network-status-banner');
    expect(banner).toBeInTheDocument();
    expect(banner).toHaveTextContent(/you are offline/i);
  });

  it('dismisses the banner automatically when connectivity is restored', () => {
    render(<CardNetworkStatusBanner />);
    goOffline();
    expect(
      screen.getByTestId('card-network-status-banner')
    ).toBeInTheDocument();
    goOnline();
    expect(
      screen.queryByTestId('card-network-status-banner')
    ).not.toBeInTheDocument();
  });

  it('disables action triggers while offline and re-enables them when online', () => {
    render(
      <CardNetworkStatusBanner>
        <button type="button">Freeze Card</button>
        <button type="button">Top Up</button>
      </CardNetworkStatusBanner>
    );
    const freeze = screen.getByRole('button', { name: 'Freeze Card' });
    const topUp = screen.getByRole('button', { name: 'Top Up' });
    expect(freeze).toBeEnabled();
    expect(topUp).toBeEnabled();

    goOffline();
    expect(freeze).toBeDisabled();
    expect(topUp).toBeDisabled();

    goOnline();
    expect(freeze).toBeEnabled();
    expect(topUp).toBeEnabled();
  });

  it('does not affect read-only content outside the action triggers', () => {
    render(
      <div>
        <p data-testid="cached-card-status">Card ending in 4567</p>
        <CardNetworkStatusBanner>
          <button type="button">Freeze Card</button>
        </CardNetworkStatusBanner>
      </div>
    );
    goOffline();
    expect(screen.getByTestId('cached-card-status')).toBeInTheDocument();
    expect(screen.getByTestId('cached-card-status')).toBeVisible();
  });
});
