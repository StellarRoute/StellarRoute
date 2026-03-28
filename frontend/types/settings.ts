import { Locale, DEFAULT_LOCALE } from '@/lib/formatting';

export type ThemeSetting = 'light' | 'dark' | 'system';

export interface Settings {
  slippageTolerance: number;
  theme: ThemeSetting;
  locale: Locale;
}

export const DEFAULT_SETTINGS: Settings = {
  slippageTolerance: 0.5,
  theme: 'system',
  locale: DEFAULT_LOCALE,
};
