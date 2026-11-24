import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { SettingsProvider } from '@/components/providers/settings-provider';
import { SettingsPageClient } from './SettingsPageClient';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock('@/hooks/useBrowserNotifications', () => ({
  useBrowserNotifications: () => ({
    browserNotifications: false,
    permissionState: 'default',
    isDisabled: false,
    enableNotifications: vi.fn(),
    disableNotifications: vi.fn(),
  }),
}));

describe('SettingsPageClient', () => {
  it('renders the settings page under the configured providers and exposes the trade and locale controls', () => {
    render(
      <SettingsProvider>
        <SettingsPageClient />
      </SettingsProvider>,
    );

    expect(
      screen.getByRole('heading', { name: 'Settings' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText('Default Slippage Tolerance (%)'),
    ).toBeInTheDocument();
    expect(screen.getByText('Language & Region')).toBeInTheDocument();

    const slippageInput = screen.getByRole('spinbutton');
    expect(slippageInput).toBeInTheDocument();
    expect(slippageInput).toHaveValue(0.5);

    const localeButton = screen.getByRole('button', {
      name: /English \(United States\)/i,
    });
    expect(localeButton).toBeInTheDocument();
  });
});
