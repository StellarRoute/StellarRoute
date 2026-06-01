'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useTheme } from 'next-themes';
import {
  Settings,
  DEFAULT_SETTINGS,
  ThemeSetting,
  AccentColor,
  FontScale,
  ACCENT_COLORS,
  FONT_SCALE_OPTIONS,
} from '@/types/settings';
import { getUserLocale } from '@/lib/formatting';

const STORAGE_KEY = 'stellar_route_settings';

interface SettingsContextType {
  settings: Settings;
  updateSlippage: (value: number) => void;
  updateTheme: (theme: ThemeSetting) => void;
  updateLocale: (locale: Settings['locale']) => void;
  /** Update the accent colour applied to primary actions (issue #521). */
  updateAccentColor: (color: AccentColor) => void;
  /** Update the root font-size multiplier (issue #522). */
  updateFontScale: (scale: FontScale) => void;
  resetSettings: () => void;
}

const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Apply the accent colour as CSS custom properties on :root so that every
 * Tailwind `text-primary` / `bg-primary` class picks it up automatically.
 */
function applyAccentColor(color: AccentColor) {
  if (typeof document === 'undefined') return;
  const hex = ACCENT_COLORS[color];
  document.documentElement.style.setProperty('--primary', hex);
  document.documentElement.style.setProperty('--ring', hex);
  document.documentElement.style.setProperty('--sidebar-primary', hex);
  document.documentElement.style.setProperty('--sidebar-ring', hex);
}

/**
 * Apply the font-scale multiplier as a CSS custom property on <html>.
 * The base size (16 px) × scale is written to `font-size` directly so that
 * every `rem` value in the UI scales proportionally (issue #522).
 */
function applyFontScale(scale: FontScale) {
  if (typeof document === 'undefined') return;
  const px = 16 * scale;
  document.documentElement.style.setProperty('font-size', `${px}px`);
}

// ── Provider ─────────────────────────────────────────────────────────────────

export function SettingsProvider({ children }: { children: ReactNode }) {
  const { theme, setTheme } = useTheme();
  const [settings, setSettings] = useState<Settings>(() => {
    if (typeof window === 'undefined') return DEFAULT_SETTINGS;
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      const parsed = stored ? (JSON.parse(stored) as Partial<Settings>) : {};
      return {
        ...DEFAULT_SETTINGS,
        ...parsed,
        theme: (theme as ThemeSetting) || parsed.theme || DEFAULT_SETTINGS.theme,
        locale: parsed.locale || getUserLocale(),
      };
    } catch (e) {
      console.error('Failed to load settings', e);
      return DEFAULT_SETTINGS;
    }
  });

  // Persist to localStorage whenever settings change
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch (e) {
      console.error('Failed to save settings', e);
    }
  }, [settings]);

  // Apply accent colour and font scale on mount and on every change
  useEffect(() => {
    applyAccentColor(settings.accentColor);
  }, [settings.accentColor]);

  useEffect(() => {
    applyFontScale(settings.fontScale);
  }, [settings.fontScale]);

  // ── Updaters ───────────────────────────────────────────────────────────────

  const isValidSlippage = (value: number) =>
    Number.isFinite(value) && value >= 0 && value <= 50;

  const updateSlippage = (value: number) => {
    if (!isValidSlippage(value)) {
      console.warn(`Ignored invalid slippage value: ${value}`);
      return;
    }
    setSettings((prev) => ({ ...prev, slippageTolerance: value }));
  };

  const updateTheme = (newTheme: ThemeSetting) => {
    setTheme(newTheme);
    setSettings((prev) => ({ ...prev, theme: newTheme }));
  };

  const updateLocale = (locale: Settings['locale']) => {
    setSettings((prev) => ({ ...prev, locale }));
  };

  const updateAccentColor = (color: AccentColor) => {
    setSettings((prev) => ({ ...prev, accentColor: color }));
  };

  const updateFontScale = (scale: FontScale) => {
    if (!FONT_SCALE_OPTIONS.includes(scale)) {
      console.warn(`Ignored invalid font scale: ${scale}`);
      return;
    }
    setSettings((prev) => ({ ...prev, fontScale: scale }));
  };

  const resetSettings = () => {
    setTheme(DEFAULT_SETTINGS.theme);
    setSettings(DEFAULT_SETTINGS);
  };

  return (
    <SettingsContext.Provider
      value={{
        settings,
        updateSlippage,
        updateTheme,
        updateLocale,
        updateAccentColor,
        updateFontScale,
        resetSettings,
      }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings() {
  const context = useContext(SettingsContext);
  if (context === undefined) {
    throw new Error('useSettings must be used within a SettingsProvider');
  }
  return context;
}

/** Returns undefined when used outside SettingsProvider instead of throwing. */
export function useOptionalSettings(): SettingsContextType | undefined {
  return useContext(SettingsContext);
}
