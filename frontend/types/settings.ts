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
  accentColor: AccentColor;
  fontScale: FontScale;
  highContrast: boolean;
}

export const DEFAULT_SETTINGS: Settings = {
  slippageTolerance: 0.5,
  theme: 'system',
  locale: DEFAULT_LOCALE,
  slippageProfiles: PRESET_SLIPPAGE_PROFILES,
  activeProfileId: 'balanced',
  accentColor: 'indigo',
  fontScale: 1,
  highContrast: false,
};

export type AccentColor = 'indigo' | 'zinc' | 'rose' | 'amber' | 'emerald' | 'cyan' | 'violet';

export const ACCENT_COLORS: Record<AccentColor, string> = {
  indigo: '#6366f1',
  zinc: '#71717a',
  rose: '#f43f5e',
  amber: '#f59e0b',
  emerald: '#10b981',
  cyan: '#06b6d4',
  violet: '#8b5cf6',
};

export type FontScale = 1 | 1.25 | 1.5 | 1.75 | 2;

export const FONT_SCALE_OPTIONS: FontScale[] = [1, 1.25, 1.5, 1.75, 2];
