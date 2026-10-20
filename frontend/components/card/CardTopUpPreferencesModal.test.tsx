import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { CardTopUpPreferencesModal } from './CardTopUpPreferencesModal';
import {
  cardTopUpPreferencesStorageKey,
  loadCardTopUpPreferences,
  saveCardTopUpPreferences,
} from '@/lib/card/preferences';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';
const OTHER_ADDRESS =
  'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB2';

if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

function openPreferences() {
  fireEvent.click(screen.getByTestId('card-preferences-gear'));
}

function thresholdField() {
  return screen.getByTestId('card-preference-input-alertThresholdUsdc');
}

function topUpField() {
  return screen.getByTestId('card-preference-input-defaultTopUpUsdc');
}

function clickPreset(field: 'alertThresholdUsdc' | 'defaultTopUpUsdc', amount: number) {
  fireEvent.click(
    screen.getByTestId(`card-preference-preset-${field}-${amount}`),
  );
}

function stored(address = ADDRESS) {
  return loadCardTopUpPreferences(address);
}

beforeEach(() => {
  window.localStorage.clear();
});

describe('CardTopUpPreferencesModal', () => {
  it('opens the settings dialog from the gear icon', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);

    openPreferences();

    expect(
      await screen.findByText('Top-up preferences'),
    ).toBeInTheDocument();
  });

  it('shows defaults when the address has no saved preferences', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();

    await screen.findByText('Top-up preferences');

    expect(thresholdField()).toHaveValue('25.00');
    expect(topUpField()).toHaveValue('50.00');
  });

  it('loads the preferences already saved for the active address', async () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 10,
      defaultTopUpUsdc: 250,
    });

    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();

    await screen.findByText('Top-up preferences');

    expect(thresholdField()).toHaveValue('10.00');
    expect(topUpField()).toHaveValue('250.00');
  });

  it('persists a preset selection to localStorage immediately', async () => {
    const onPreferencesChange = vi.fn();
    render(
      <CardTopUpPreferencesModal
        stellarAddress={ADDRESS}
        onPreferencesChange={onPreferencesChange}
      />,
    );
    openPreferences();
    await screen.findByText('Top-up preferences');

    clickPreset('alertThresholdUsdc', 100);

    expect(stored()).toEqual({ alertThresholdUsdc: 100, defaultTopUpUsdc: 50 });
    expect(
      window.localStorage.getItem(cardTopUpPreferencesStorageKey(ADDRESS)),
    ).toBe(JSON.stringify({ alertThresholdUsdc: 100, defaultTopUpUsdc: 50 }));
    await waitFor(() =>
      expect(onPreferencesChange).toHaveBeenCalledWith({
        alertThresholdUsdc: 100,
        defaultTopUpUsdc: 50,
      }),
    );
  });

  it('keeps the other field when a preset is selected', async () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 25,
      defaultTopUpUsdc: 10,
    });

    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    clickPreset('defaultTopUpUsdc', 50);

    expect(stored()).toEqual({ alertThresholdUsdc: 25, defaultTopUpUsdc: 50 });
  });

  it('saves a custom threshold and top-up amount', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    fireEvent.change(thresholdField(), { target: { value: '12.50' } });
    fireEvent.change(topUpField(), { target: { value: '75' } });
    fireEvent.click(screen.getByTestId('card-preferences-save'));

    expect(stored()).toEqual({ alertThresholdUsdc: 12.5, defaultTopUpUsdc: 75 });
  });

  it('rejects non-positive and non-numeric amounts without saving', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    fireEvent.change(thresholdField(), { target: { value: '0' } });
    fireEvent.change(topUpField(), { target: { value: '-5' } });
    fireEvent.click(screen.getByTestId('card-preferences-save'));

    expect(
      screen.getByTestId('card-preference-error-alertThresholdUsdc'),
    ).toHaveTextContent(/greater than 0/);
    expect(
      screen.getByTestId('card-preference-error-defaultTopUpUsdc'),
    ).toHaveTextContent(/greater than 0/);
    expect(thresholdField()).toHaveAttribute('aria-invalid', 'true');
    expect(stored()).toBeNull();
    expect(
      window.localStorage.getItem(cardTopUpPreferencesStorageKey(ADDRESS)),
    ).toBeNull();

    fireEvent.change(thresholdField(), { target: { value: 'twenty five' } });
    fireEvent.click(screen.getByTestId('card-preferences-save'));

    expect(stored()).toBeNull();
  });

  it('keeps a valid record when only one field is invalid', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    fireEvent.change(thresholdField(), { target: { value: '' } });
    fireEvent.change(topUpField(), { target: { value: '90' } });
    fireEvent.click(screen.getByTestId('card-preferences-save'));

    expect(
      screen.getByTestId('card-preference-error-alertThresholdUsdc'),
    ).toBeInTheDocument();
    expect(stored()).toBeNull();

    fireEvent.change(thresholdField(), { target: { value: '30' } });
    fireEvent.click(screen.getByTestId('card-preferences-save'));

    expect(stored()).toEqual({ alertThresholdUsdc: 30, defaultTopUpUsdc: 90 });
  });

  it('reset to defaults restores 25 USDC threshold and 50 USDC top-up', async () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 100,
      defaultTopUpUsdc: 500,
    });

    render(<CardTopUpPreferencesModal stellarAddress={ADDRESS} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    fireEvent.click(screen.getByTestId('card-preferences-reset'));

    expect(stored()).toEqual({ alertThresholdUsdc: 25, defaultTopUpUsdc: 50 });
    expect(thresholdField()).toHaveValue('25.00');
    expect(topUpField()).toHaveValue('50.00');
  });

  it('isolates reads and writes per Stellar address', async () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 100,
      defaultTopUpUsdc: 100,
    });
    saveCardTopUpPreferences(OTHER_ADDRESS, {
      alertThresholdUsdc: 10,
      defaultTopUpUsdc: 250,
    });

    const { rerender } = render(
      <CardTopUpPreferencesModal stellarAddress={ADDRESS} />,
    );
    openPreferences();
    await screen.findByText('Top-up preferences');

    clickPreset('alertThresholdUsdc', 25);
    expect(stored(ADDRESS)).toEqual({
      alertThresholdUsdc: 25,
      defaultTopUpUsdc: 100,
    });

    rerender(<CardTopUpPreferencesModal stellarAddress={OTHER_ADDRESS} />);
    await waitFor(() => expect(thresholdField()).toHaveValue('10.00'));
    expect(topUpField()).toHaveValue('250.00');

    clickPreset('defaultTopUpUsdc', 50);

    expect(stored(OTHER_ADDRESS)).toEqual({
      alertThresholdUsdc: 10,
      defaultTopUpUsdc: 50,
    });
    expect(stored(ADDRESS)).toEqual({
      alertThresholdUsdc: 25,
      defaultTopUpUsdc: 100,
    });
  });

  it('asks for a wallet connection and blocks saving when disconnected', async () => {
    render(<CardTopUpPreferencesModal stellarAddress={null} />);
    openPreferences();
    await screen.findByText('Top-up preferences');

    expect(
      screen.getByTestId('card-preferences-disconnected'),
    ).toBeInTheDocument();
    expect(thresholdField()).toBeDisabled();
    expect(topUpField()).toBeDisabled();
    expect(screen.getByTestId('card-preferences-save')).toBeDisabled();

    fireEvent.click(screen.getByTestId('card-preferences-reset'));

    expect(window.localStorage.length).toBe(0);
  });
});
