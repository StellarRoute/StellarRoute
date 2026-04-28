import { Locale, DEFAULT_LOCALE } from '@/lib/formatting';

export type ThemeSetting = 'light' | 'dark' | 'system';

export interface SlippageProfile {
  id: string;
  name: string;
  value: number;
  isPreset: boolean;
}

export const PRESET_SLIPPAGE_PROFILES: SlippageProfile[] = [
  { id: 'safe', name: 'Safe', value: 0.1, isPreset: true },
  { id: 'balanced', name: 'Balanced', value: 0.5, isPreset: true },
  { id: 'aggressive', name: 'Aggressive', value: 1.0, isPreset: true },
];

export interface Settings {
  slippageTolerance: number;
  theme: ThemeSetting;
  locale: Locale;
  slippageProfiles: SlippageProfile[];
  activeProfileId: string;
}

export const DEFAULT_SETTINGS: Settings = {
  slippageTolerance: 0.5,
  theme: 'system',
  locale: DEFAULT_LOCALE,
  slippageProfiles: PRESET_SLIPPAGE_PROFILES,
  activeProfileId: 'balanced',
};
