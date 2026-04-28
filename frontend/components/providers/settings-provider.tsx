'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useTheme } from 'next-themes';
import { Settings, DEFAULT_SETTINGS, ThemeSetting, SlippageProfile } from '@/types/settings';
import { getUserLocale } from '@/lib/formatting';

const STORAGE_KEY = 'stellar_route_settings';

interface SettingsContextType {
  settings: Settings;
  updateSlippage: (value: number) => void;
  updateTheme: (theme: ThemeSetting) => void;
  updateLocale: (locale: Settings['locale']) => void;
  resetSettings: () => void;
  addProfile: (profile: { name: string; value: number }) => void;
  updateProfile: (id: string, updates: Partial<SlippageProfile>) => void;
  deleteProfile: (id: string) => void;
  selectProfile: (id: string) => void;
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

  const addProfile = (profile: { name: string; value: number }) => {
    if (!isValidSlippage(profile.value)) return;
    const newProfile: SlippageProfile = {
      id: crypto.randomUUID(),
      name: profile.name,
      value: profile.value,
      isPreset: false,
    };
    setSettings((prev) => ({
      ...prev,
      slippageProfiles: [...prev.slippageProfiles, newProfile],
      activeProfileId: newProfile.id,
      slippageTolerance: newProfile.value,
    }));
  };

  const updateProfile = (id: string, updates: Partial<SlippageProfile>) => {
    if (updates.value !== undefined && !isValidSlippage(updates.value)) return;
    setSettings((prev) => ({
      ...prev,
      slippageProfiles: prev.slippageProfiles.map((p) =>
        p.id === id && !p.isPreset ? { ...p, ...updates } : p
      ),
      slippageTolerance: prev.activeProfileId === id && updates.value !== undefined ? updates.value : prev.slippageTolerance,
    }));
  };

  const deleteProfile = (id: string) => {
    setSettings((prev) => {
      const profile = prev.slippageProfiles.find((p) => p.id === id);
      if (profile?.isPreset) return prev; // Cannot delete preset
      
      const newProfiles = prev.slippageProfiles.filter((p) => p.id !== id);
      let newActiveId = prev.activeProfileId;
      let newSlippage = prev.slippageTolerance;
      
      if (prev.activeProfileId === id) {
        newActiveId = DEFAULT_SETTINGS.activeProfileId;
        const fallback = newProfiles.find((p) => p.id === newActiveId);
        newSlippage = fallback ? fallback.value : DEFAULT_SETTINGS.slippageTolerance;
      }

      return {
        ...prev,
        slippageProfiles: newProfiles,
        activeProfileId: newActiveId,
        slippageTolerance: newSlippage,
      };
    });
  };

  const selectProfile = (id: string) => {
    setSettings((prev) => {
      const profile = prev.slippageProfiles.find((p) => p.id === id);
      if (!profile) return prev;
      return {
        ...prev,
        activeProfileId: id,
        slippageTolerance: profile.value,
      };
    });
  };

  return (
    <SettingsContext.Provider
      value={{
        settings,
        updateSlippage,
        updateTheme,
        updateLocale,
        resetSettings,
        addProfile,
        updateProfile,
        deleteProfile,
        selectProfile,
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
