import { describe, it, expect } from 'vitest';
import {
  formatNumber,
  formatAmount,
  formatPrice,
  formatPercentage,
  formatCurrency,
  formatCompactAmount,
  formatRate,
  formatFee,
  getUserLocale,
  DEFAULT_LOCALE,
  SUPPORTED_LOCALES,
  type Locale
} from './formatting';

describe('formatting utilities', () => {
  describe('formatNumber', () => {
    it('should format basic numbers with default locale', () => {
      expect(formatNumber(1234.56)).toBe('1,234.56');
      expect(formatNumber('1234.56')).toBe('1,234.56');
    });

    it('should format numbers with different locales', () => {
      expect(formatNumber(1234.56, 'de-DE')).toBe('1.234,56');
      expect(formatNumber(1234.56, 'fr-FR')).toBe('1 234,56');
    });

    it('should handle different precision settings', () => {
      expect(formatNumber(1234.56789, 'en-US', { maximumFractionDigits: 4 })).toBe('1,234.5679');
      expect(formatNumber(1234.56789, 'en-US', { minimumFractionDigits: 2 })).toBe('1,234.56789');
    });

    it('should handle zero and negative numbers', () => {
      expect(formatNumber(0)).toBe('0');
      expect(formatNumber(-1234.56)).toBe('-1,234.56');
    });

    it('should handle invalid numbers gracefully', () => {
      expect(formatNumber(NaN)).toBe('0');
      expect(formatNumber(Infinity)).toBe('0');
      expect(formatNumber('invalid')).toBe('0');
    });

    it('should respect grouping settings', () => {
      expect(formatNumber(1234567, 'en-US', { showGrouping: false })).toBe('1234567');
      expect(formatNumber(1234567, 'en-US', { showGrouping: true })).toBe('1,234,567');
    });
  });

  describe('formatAmount', () => {
    it('should format cryptocurrency amounts with appropriate precision', () => {
      expect(formatAmount(1234.56789, 'en-US', 7)).toBe('1,234.56789');
      expect(formatAmount(0.00000123, 'en-US', 7)).toBe('0.0000012');
    });

    it('should adjust precision for very small amounts', () => {
      expect(formatAmount(0.00001, 'en-US', 7)).toBe('0.00001');
      expect(formatAmount(0.0000001, 'en-US', 7)).toBe('0.0000001');
    });

    it('should reduce decimals for large amounts', () => {
      expect(formatAmount(1500, 'en-US', 7)).toBe('1,500');
      expect(formatAmount(1234.567, 'en-US', 7)).toBe('1,234.567');
    });

    it('should handle different locales', () => {
      expect(formatAmount(1234.56789, 'de-DE', 7)).toBe('1.234,56789');
      expect(formatAmount(1234.56789, 'fr-FR', 7)).toBe('1 234,56789');
    });
  });

  describe('formatPrice', () => {
    it('should format prices with appropriate precision', () => {
      expect(formatPrice(0.123456, 'en-US')).toBe('0.123456');
      expect(formatPrice(1.234567, 'en-US')).toBe('1.23');
      expect(formatPrice(1234.567, 'en-US')).toBe('1,235');
    });

    it('should handle very small prices', () => {
      expect(formatPrice(0.001, 'en-US')).toBe('0.001000');
      expect(formatPrice(0.0001, 'en-US')).toBe('0.000100');
    });

    it('should respect explicit precision settings', () => {
      expect(formatPrice(1.234567, 'en-US', { precision: 4 })).toBe('1.2346');
      expect(formatPrice(1.234567, 'en-US', { precision: 2 })).toBe('1.23');
    });

    it('should handle different locales', () => {
      expect(formatPrice(1.234567, 'de-DE')).toBe('1,23');
      expect(formatPrice(1.234567, 'fr-FR')).toBe('1,23');
    });
  });

  describe('formatPercentage', () => {
    it('should format percentages correctly', () => {
      expect(formatPercentage(0.1234, 'en-US')).toBe('12.34%');
      expect(formatPercentage(0.01234, 'en-US')).toBe('1.23%');
      expect(formatPercentage(1.2345, 'en-US')).toBe('123.45%');
    });

    it('should handle different locales', () => {
      expect(formatPercentage(0.1234, 'de-DE')).toBe('12,34%');
      expect(formatPercentage(0.1234, 'fr-FR')).toBe('12,34%');
    });

    it('should respect precision settings', () => {
      expect(formatPercentage(0.123456, 'en-US', { maximumFractionDigits: 3 })).toBe('12.346%');
      expect(formatPercentage(0.123456, 'en-US', { minimumFractionDigits: 1 })).toBe('12.35%');
    });
  });

  describe('formatCurrency', () => {
    it('should format currency amounts', () => {
      expect(formatCurrency(1234.56, 'en-US', 'USD')).toBe('$1,234.56');
      expect(formatCurrency(1234.56, 'en-GB', 'GBP')).toBe('£1,234.56');
      expect(formatCurrency(1234.56, 'de-DE', 'EUR')).toBe('1.234,56 €');
    });

    it('should handle different currencies', () => {
      expect(formatCurrency(1234.56, 'en-US', 'JPY')).toBe('¥1,235');
      expect(formatCurrency(1234.56, 'en-US', 'EUR')).toBe('€1,234.56');
    });
  });

  describe('formatCompactAmount', () => {
    it('should use compact notation for large numbers', () => {
      expect(formatCompactAmount(1500000, 'en-US')).toBe('1.5M');
      expect(formatCompactAmount(1234567890, 'en-US')).toBe('1.23B');
    });

    it('should use regular formatting for smaller numbers', () => {
      expect(formatCompactAmount(1234.56, 'en-US')).toBe('1,234.56');
      expect(formatCompactAmount(999999, 'en-US')).toBe('999,999');
    });

    it('should handle different locales', () => {
      expect(formatCompactAmount(1500000, 'de-DE')).toBe('1,5 Mio.');
    });
  });

  describe('formatRate', () => {
    it('should format exchange rates', () => {
      expect(formatRate(1, 'XLM', 0.98, 'USDC', 'en-US')).toBe('1 XLM ≈ 0.98 USDC');
      expect(formatRate(1, 'BTC', 45000, 'USD', 'en-US')).toBe('1 BTC ≈ 45,000.00 USD');
    });

    it('should handle different locales', () => {
      expect(formatRate(1, 'XLM', 0.98, 'USDC', 'de-DE')).toBe('1 XLM ≈ 0,98 USDC');
      expect(formatRate(1, 'XLM', 0.98, 'USDC', 'fr-FR')).toBe('1 XLM ≈ 0,98 USDC');
    });
  });

  describe('formatFee', () => {
    it('should format fee amounts', () => {
      expect(formatFee(0.01, 'XLM', 'en-US')).toBe('0.01 XLM');
      expect(formatFee(0.015, 'ETH', 'en-US')).toBe('0.015 ETH');
    });

    it('should handle different locales', () => {
      expect(formatFee(0.01, 'XLM', 'de-DE')).toBe('0,01 XLM');
      expect(formatFee(0.01, 'XLM', 'fr-FR')).toBe('0,01 XLM');
    });
  });

  describe('getUserLocale', () => {
    it('should return default locale when navigator is unavailable', () => {
      // Mock window being undefined
      const originalWindow = global.window;
      delete (global as any).window;
      
      expect(getUserLocale()).toBe(DEFAULT_LOCALE);
      
      // Restore window
      global.window = originalWindow;
    });

    it('should return supported browser locale', () => {
      // Mock navigator.language
      const originalNavigator = global.navigator;
      (global.navigator as any) = { language: 'en-US' };
      
      expect(getUserLocale()).toBe('en-US');
      
      // Restore navigator
      global.navigator = originalNavigator;
    });

    it('should fallback to language part for unsupported locales', () => {
      const originalNavigator = global.navigator;
      (global.navigator as any) = { language: 'en-CA' };
      
      expect(getUserLocale()).toBe('en-US');
      
      global.navigator = originalNavigator;
    });

    it('should return default locale for unsupported languages', () => {
      const originalNavigator = global.navigator;
      (global.navigator as any) = { language: 'unsupported-locale' };
      
      expect(getUserLocale()).toBe(DEFAULT_LOCALE);
      
      global.navigator = originalNavigator;
    });
  });

  describe('constants and types', () => {
    it('should have correct supported locales', () => {
      expect(Object.keys(SUPPORTED_LOCALES)).toContain('en-US');
      expect(Object.keys(SUPPORTED_LOCALES)).toContain('de-DE');
      expect(Object.keys(SUPPORTED_LOCALES)).toContain('fr-FR');
      expect(Object.keys(SUPPORTED_LOCALES)).toContain('ja-JP');
    });

    it('should have default locale', () => {
      expect(DEFAULT_LOCALE).toBe('en-US');
    });
  });

  describe('edge cases', () => {
    it('should handle string inputs that represent numbers', () => {
      expect(formatAmount('1234.56789', 'en-US', 7)).toBe('1,234.56789');
      expect(formatPrice('0.123456', 'en-US')).toBe('0.123456');
      expect(formatPercentage('0.1234', 'en-US')).toBe('12.34%');
    });

    it('should handle zero values', () => {
      expect(formatAmount(0, 'en-US', 7)).toBe('0');
      expect(formatPrice(0, 'en-US')).toBe('0');
      expect(formatPercentage(0, 'en-US')).toBe('0.00%');
    });

    it('should handle very large numbers', () => {
      expect(formatAmount(999999999999, 'en-US', 7)).toBe('999,999,999,999');
      expect(formatCompactAmount(999999999999, 'en-US')).toBe('1T');
    });

    it('should handle very small numbers', () => {
      expect(formatAmount(0.0000001, 'en-US', 7)).toBe('0.0000001');
      expect(formatPrice(0.000001, 'en-US')).toBe('0.000001');
    });
  });
});
