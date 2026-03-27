'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useTheme } from 'next-themes';
import { Settings, DEFAULT_SETTINGS, ThemeSetting } from '@/types/settings';
import { getUserLocale } from '@/lib/formatting';

const STORAGE_KEY = 'stellar_route_settings';

interface SettingsContextType {
  settings: Settings;
  updateSlippage: (value: number) => void;
  updateTheme: (theme: ThemeSetting) => void;
  updateLocale: (locale: Settings['locale']) => void;
  resetSettings: () => void;
}

const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

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

  // Sync with next-themes theme
  useEffect(() => {
    if (theme) {
      setSettings((prev) => ({ ...prev, theme: theme as ThemeSetting }));
    }
  }, [theme]);

  // Handle local storage saving
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch (e) {
      console.error('Failed to save settings', e);
    }
  }, [settings]);

  const isValidSlippage = (value: number) => Number.isFinite(value) && value >= 0 && value <= 50;

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
