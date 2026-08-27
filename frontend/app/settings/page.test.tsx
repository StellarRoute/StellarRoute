import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import SettingsPage from './page';
import { SettingsProvider } from '@/components/providers/settings-provider';

// Mock notification hook
vi.mock('@/hooks/useBrowserNotifications', () => ({
  useBrowserNotifications: () => ({
    browserNotifications: false,
    permissionState: 'default',
    isDisabled: false,
    enableNotifications: vi.fn(),
    disableNotifications: vi.fn(),
  }),
}));

describe('SettingsPage', () => {
  it('renders settings title and feature flags help card', () => {
    render(
      <SettingsProvider>
        <SettingsPage />
      </SettingsProvider>
    );

    expect(screen.getByRole('heading', { name: /settings/i, level: 1 })).toBeInTheDocument();
    expect(screen.getByTestId('settings-feature-flags-help')).toBeInTheDocument();
    expect(screen.getByText(/Feature Flags & URL Reference/i)).toBeInTheDocument();
    expect(screen.getByText('/settings')).toBeInTheDocument();
    expect(screen.getByText('routes_beta')).toBeInTheDocument();
    expect(screen.getByText('real_xdr')).toBeInTheDocument();
  });
});
