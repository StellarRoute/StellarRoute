import React from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemePreview } from '@/components/settings/ThemePreview';
import type { ThemeSetting } from '@/types/settings';

// ── ThemePreview unit tests ───────────────────────────────────────────────────
describe('ThemePreview component', () => {
  afterEach(cleanup);

  it.each<ThemeSetting>(['light', 'dark', 'system'])(
    'renders a swatch with aria-label for %s theme',
    (theme) => {
      render(<ThemePreview previewTheme={theme} />);
      const preview = screen.getByTestId('theme-preview');
      expect(preview).toBeInTheDocument();
      expect(preview).toHaveAttribute('aria-label', expect.stringContaining(theme === 'light' ? 'Light' : theme === 'dark' ? 'Dark' : 'System'));
    }
  );

  it('renders an aria-hidden decorative swatch div', () => {
    render(<ThemePreview previewTheme="dark" />);
    const swatch = screen.getByTestId('theme-preview').querySelector('[aria-hidden="true"]');
    expect(swatch).toBeInTheDocument();
  });
});

// ── SettingsPageClient: theme preview integration via mocked flag ─────────────
// The Radix Select does not work in jsdom (missing hasPointerCapture).
// We test the flag-gated preview behaviour by mounting SettingsPageClient with a
// stubbed Select that calls onValueChange directly.

vi.mock('@/components/ui/select', () => ({
  Select: ({ value, onValueChange, children }: {
    value: string;
    onValueChange?: (v: string) => void;
    children: React.ReactNode;
  }) => (
    <div>
      <select
        data-testid="theme-select"
        value={value}
        onChange={(e) => onValueChange?.(e.target.value)}
      >
        <option value="light">Light</option>
        <option value="dark">Dark</option>
        <option value="system">System</option>
      </select>
      {children}
    </div>
  ),
  SelectTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  SelectValue: ({ placeholder }: { placeholder?: string }) => <span>{placeholder}</span>,
  SelectContent: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  SelectItem: ({ value, children }: { value: string; children: React.ReactNode }) => (
    <option value={value}>{children}</option>
  ),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn(), message: vi.fn() },
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

const mockUseFeatureFlag = vi.fn((flag: string) => ({
  enabled: flag === 'settings_theme_preview',
  loading: false,
}));
vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: (flag: string) => mockUseFeatureFlag(flag),
}));

import { SettingsProvider } from '@/components/providers/settings-provider';
import { SettingsPageClient } from './SettingsPageClient';

function Wrapper({ children }: { children: React.ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

describe('Settings theme preview (flag on)', () => {
  beforeEach(() => {
    mockUseFeatureFlag.mockImplementation((flag: string) => ({
      enabled: flag === 'settings_theme_preview',
      loading: false,
    }));
  });
  afterEach(cleanup);

  it('shows ThemePreview swatch when user picks a different theme', async () => {
    const user = userEvent.setup();
    render(<SettingsPageClient />, { wrapper: Wrapper });

    const sel = screen.getByTestId('theme-select');
    await user.selectOptions(sel, 'dark');

    expect(screen.getByTestId('theme-preview')).toBeInTheDocument();
    expect(screen.getByTestId('theme-preview')).toHaveAttribute('aria-label', 'Dark theme preview');
  });

  it('shows Apply theme button while a pending theme is selected', async () => {
    const user = userEvent.setup();
    render(<SettingsPageClient />, { wrapper: Wrapper });

    const sel = screen.getByTestId('theme-select');
    await user.selectOptions(sel, 'light');

    expect(screen.getByTestId('theme-preview-apply')).toBeInTheDocument();
  });

  it('removes preview and Apply button after Apply is clicked', async () => {
    const user = userEvent.setup();
    render(<SettingsPageClient />, { wrapper: Wrapper });

    const sel = screen.getByTestId('theme-select');
    await user.selectOptions(sel, 'dark');

    await user.click(screen.getByTestId('theme-preview-apply'));

    expect(screen.queryByTestId('theme-preview')).not.toBeInTheDocument();
    expect(screen.queryByTestId('theme-preview-apply')).not.toBeInTheDocument();
  });
});

describe('Settings theme preview (flag off)', () => {
  beforeEach(() => {
    mockUseFeatureFlag.mockReturnValue({ enabled: false, loading: false });
  });
  afterEach(cleanup);

  it('applies theme immediately with no preview swatch when flag is off', async () => {
    const user = userEvent.setup();
    render(<SettingsPageClient />, { wrapper: Wrapper });

    const sel = screen.getByTestId('theme-select');
    await user.selectOptions(sel, 'dark');

    expect(screen.queryByTestId('theme-preview')).not.toBeInTheDocument();
    expect(screen.queryByTestId('theme-preview-apply')).not.toBeInTheDocument();
  });
});
