import { Locale, DEFAULT_LOCALE } from '@/lib/formatting';

export type ThemeSetting = 'light' | 'dark' | 'system';

/**
 * Preset accent colour tokens available in the colour picker (issue #521).
 * Each value maps to a CSS hex colour that is applied to --primary.
 */
export const ACCENT_COLORS = {
  indigo: '#6366f1',
  sky: '#0ea5e9',
  emerald: '#10b981',
  rose: '#f43f5e',
  amber: '#f59e0b',
  violet: '#8b5cf6',
} as const;

export type AccentColor = keyof typeof ACCENT_COLORS;

export const DEFAULT_ACCENT_COLOR: AccentColor = 'indigo';

/**
 * Font scale setting (issue #522).
 * Represents a multiplier applied to the root font size.
 * Range: 1.0 (100%) to 2.0 (200%), step 0.25.
 */
export type FontScale = 1.0 | 1.25 | 1.5 | 1.75 | 2.0;

export const FONT_SCALE_OPTIONS: FontScale[] = [1.0, 1.25, 1.5, 1.75, 2.0];

export const DEFAULT_FONT_SCALE: FontScale = 1.0;

export interface Settings {
  slippageTolerance: number;
  theme: ThemeSetting;
  locale: Locale;
  /** User-chosen accent colour applied to primary actions (issue #521). */
  accentColor: AccentColor;
  /**
   * Root font-size multiplier for accessibility (issue #522).
   * 1.0 = 100% (default), 2.0 = 200%.
   */
  fontScale: FontScale;
}

export const DEFAULT_SETTINGS: Settings = {
  slippageTolerance: 0.5,
  theme: 'system',
  locale: DEFAULT_LOCALE,
  accentColor: DEFAULT_ACCENT_COLOR,
  fontScale: DEFAULT_FONT_SCALE,
};
