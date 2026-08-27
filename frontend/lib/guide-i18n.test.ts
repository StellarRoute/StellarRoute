import { describe, expect, it } from 'vitest';
import {
  createGuideTranslator,
  resolveGuideLocale,
  GUIDE_FALLBACK_LOCALE,
  GuideTranslationKey,
} from './guide-i18n';

describe('guide-i18n', () => {
  it('resolves default and alias locales', () => {
    expect(resolveGuideLocale(null)).toBe('en-US');
    expect(resolveGuideLocale('en-GB')).toBe('en-US');
    expect(resolveGuideLocale('es-ES')).toBe('es-ES');
    expect(resolveGuideLocale('zh-CN')).toBe('zh-CN');
  });

  it('translates en-US guide headings and CTAs', () => {
    const translator = createGuideTranslator('en-US');
    expect(translator.t('guide.header.title')).toBe('Your first live swap');
    expect(translator.t('guide.cta.openSwap')).toBe('Open swap');
    expect(translator.t('guide.cta.fullGuide')).toBe('Full guide on GitHub');
    expect(translator.t('guide.step.label', { number: 1 })).toBe('Step 1');
    expect(translator.t('guide.step1.title')).toBe('Connect your wallet');
    expect(translator.t('guide.step6.title')).toBe('Confirm in your wallet');
  });

  it('interpolates variables in guide strings', () => {
    const translator = createGuideTranslator('en-US');
    expect(translator.t('guide.step.label', { number: 3 })).toBe('Step 3');
  });

  it('falls back gracefully to en-US for unsupported/missing keys', () => {
    const translator = createGuideTranslator('es-ES');
    expect(translator.t('guide.cta.openSwap')).toBe('Abrir intercambio');
    // If a non-existent key is passed, returns the key
    // @ts-expect-error test fallback for unknown key
    expect(translator.t('guide.unknown.key')).toBe('guide.unknown.key');
  });
});
