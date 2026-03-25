import { describe, expect, it } from 'vitest';

import {
  clampToMaxDecimals,
  containsScientificNotation,
  DEFAULT_ISSUED_MAX_DECIMALS,
  formatMaxAmountForInput,
  fractionalDigitCount,
  maxDecimalsForSellAsset,
  normalizeDecimalString,
  parseSellAmount,
  STELLAR_NATIVE_MAX_DECIMALS,
  stripAmountWhitespace,
  trimTrailingZeros,
} from './amount-input';

describe('stripAmountWhitespace', () => {
  it('removes spaces and NBSP variants', () => {
    expect(stripAmountWhitespace(' 1\u00A02\u202F3 ')).toBe('123');
  });
});

describe('containsScientificNotation', () => {
  it('detects scientific notation', () => {
    expect(containsScientificNotation('1e-8')).toBe(true);
    expect(containsScientificNotation('2E+10')).toBe(true);
    expect(containsScientificNotation('10')).toBe(false);
    expect(containsScientificNotation('1.0')).toBe(false);
  });
});

describe('normalizeDecimalString', () => {
  it('parses US-style thousands and decimal', () => {
    expect(normalizeDecimalString('1,234.56')).toBe('1234.56');
  });

  it('parses EU-style thousands and decimal', () => {
    expect(normalizeDecimalString('1.234,56')).toBe('1234.56');
  });

  it('treats lone comma as decimal separator', () => {
    expect(normalizeDecimalString('0,5')).toBe('0.5');
  });

  it('allows leading-dot fractional form', () => {
    expect(normalizeDecimalString('.5')).toBe('.5');
  });

  it('rejects scientific notation', () => {
    expect(normalizeDecimalString('1e6')).toBeNull();
  });

  it('returns null for empty after strip', () => {
    expect(normalizeDecimalString('   ')).toBeNull();
  });

  it('handles paste with thin spaces', () => {
    expect(normalizeDecimalString('12\u202F345,67')).toBe('12345.67');
  });
});

describe('fractionalDigitCount', () => {
  it('counts digits after decimal', () => {
    expect(fractionalDigitCount('1.234')).toBe(3);
    expect(fractionalDigitCount('10')).toBe(0);
  });
});

describe('maxDecimalsForSellAsset', () => {
  it('uses native precision', () => {
    expect(maxDecimalsForSellAsset('native')).toBe(STELLAR_NATIVE_MAX_DECIMALS);
  });

  it('defaults issued assets', () => {
    expect(maxDecimalsForSellAsset('USDC:ISSUER')).toBe(
      DEFAULT_ISSUED_MAX_DECIMALS,
    );
  });

  it('respects explicit API decimals', () => {
    expect(maxDecimalsForSellAsset('native', 4)).toBe(4);
  });
});

describe('parseSellAmount', () => {
  const d7 = 7;

  it('returns empty for blank input', () => {
    expect(parseSellAmount('', d7).status).toBe('empty');
  });

  it('accepts valid amounts', () => {
    const r = parseSellAmount('12.5', d7);
    expect(r.status).toBe('ok');
    if (r.status === 'ok') {
      expect(r.normalized).toBe('12.5');
      expect(r.numeric).toBe(12.5);
    }
  });

  it('rejects non-positive', () => {
    expect(parseSellAmount('0', d7).status).toBe('invalid');
    expect(parseSellAmount('-1', d7).status).toBe('invalid');
  });

  it('rejects excess precision', () => {
    const r = parseSellAmount('1.123456789', 7);
    expect(r.status).toBe('precision_exceeded');
    if (r.status === 'precision_exceeded') {
      expect(r.exceededBy).toBeGreaterThan(0);
    }
  });
});

describe('clampToMaxDecimals', () => {
  it('truncates fractional digits', () => {
    expect(clampToMaxDecimals('1.123456789', 7)).toBe('1.1234567');
  });
});

describe('trimTrailingZeros', () => {
  it('strips trailing fractional zeros', () => {
    expect(trimTrailingZeros('10.500')).toBe('10.5');
    expect(trimTrailingZeros('10.00')).toBe('10');
  });
});

describe('formatMaxAmountForInput', () => {
  it('formats balance for input field', () => {
    expect(formatMaxAmountForInput('1234.5670000', 7)).toBe('1234.567');
  });
});
