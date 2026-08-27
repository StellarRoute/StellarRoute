import { describe, expect, it } from 'vitest';
import {
  createOfframpTranslator,
  resolveOfframpLocale,
  OFFRAMP_FALLBACK_LOCALE,
  OfframpTranslationKey,
} from './offramp-i18n';

describe('offramp-i18n', () => {
  it('resolves locales and aliases properly', () => {
    expect(resolveOfframpLocale(null)).toBe('en-US');
    expect(resolveOfframpLocale('en-GB')).toBe('en-US');
    expect(resolveOfframpLocale('es-ES')).toBe('es-ES');
    expect(resolveOfframpLocale('zh-CN')).toBe('zh-CN');
  });

  it('translates en-US offramp keys', () => {
    const translator = createOfframpTranslator('en-US');
    expect(translator.t('offramp.hero.title')).toBe('Stablecoin to local fiat');
    expect(translator.t('offramp.mode.directTitle')).toBe('Stellar USDC');
    expect(translator.t('offramp.form.amountLabel')).toBe('Amount');
    expect(translator.t('offramp.destination.title')).toBe('You receive');
    expect(translator.t('offramp.destination.bankLabel')).toBe('Bank');
    expect(translator.t('offramp.destination.accountNumberLabel')).toBe('Account number');
  });

  it('interpolates variables into offramp strings', () => {
    const translator = createOfframpTranslator('en-US');
    expect(
      translator.t('offramp.ready.title', { amount: '157,210.00' })
    ).toBe('Route ready · ₦157,210.00 indicative');
    expect(
      translator.t('offramp.form.onChain', { chain: 'Stellar' })
    ).toBe('On Stellar');
  });

  it('falls back to en-US when requested', () => {
    const translator = createOfframpTranslator('es-ES');
    expect(translator.t('offramp.mode.directBadge')).toBe('Más rápido');
  });
});
