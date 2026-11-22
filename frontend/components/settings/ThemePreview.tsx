'use client';

import { ThemeSetting } from '@/types/settings';

const SWATCH: Record<ThemeSetting, { bg: string; text: string; label: string }> = {
  light: { bg: '#ffffff', text: '#0f172a', label: 'Light' },
  dark: { bg: '#0f172a', text: '#f8fafc', label: 'Dark' },
  system: { bg: 'linear-gradient(135deg, #ffffff 50%, #0f172a 50%)', text: '#6b7280', label: 'System' },
};

interface ThemePreviewProps {
  /** The theme being previewed (not yet applied). */
  previewTheme: ThemeSetting;
}

/**
 * Renders a small visual swatch for the given theme before the user applies it.
 * Used by the settings page when the `settings_theme_preview` flag is on.
 */
export function ThemePreview({ previewTheme }: ThemePreviewProps) {
  const swatch = SWATCH[previewTheme];

  return (
    <div
      data-testid="theme-preview"
      aria-label={`${swatch.label} theme preview`}
      className="flex items-center gap-3 rounded-lg border px-4 py-3"
    >
      <div
        aria-hidden="true"
        className="h-10 w-16 shrink-0 rounded-md border"
        style={{ background: swatch.bg }}
      />
      <span className="text-sm text-muted-foreground">
        Preview: <strong>{swatch.label}</strong>
      </span>
    </div>
  );
}
