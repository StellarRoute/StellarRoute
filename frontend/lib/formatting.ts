/**
 * Localized number formatting utilities for amounts and prices.
 * Supports various locales with proper decimal separators, thousand separators,
 * and currency formatting.
 */

export type Locale = 'en-US' | 'en-GB' | 'de-DE' | 'fr-FR' | 'es-ES' | 'ja-JP' | 'zh-CN';

export interface NumberFormatOptions {
  minimumFractionDigits?: number;
  maximumFractionDigits?: number;
  showGrouping?: boolean;
  style?: 'decimal' | 'currency';
  currency?: string;
}

export interface PriceFormatOptions extends NumberFormatOptions {
  precision?: number; // For crypto-specific precision
}

/**
 * Default locale fallback
 */
export const DEFAULT_LOCALE: Locale = 'en-US';

/**
 * Supported locales with their display names
 */
export const SUPPORTED_LOCALES: Record<Locale, string> = {
  'en-US': 'English (United States)',
  'en-GB': 'English (United Kingdom)', 
  'de-DE': 'Deutsch (Deutschland)',
  'fr-FR': 'Français (France)',
  'es-ES': 'Español (España)',
  'ja-JP': '日本語',
  'zh-CN': '中文 (简体)'
};

/**
 * Format a number with locale-specific formatting
 */
export function formatNumber(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  options: NumberFormatOptions = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0';
  }

  const {
    minimumFractionDigits = 0,
    maximumFractionDigits = 20,
    showGrouping = true,
    style = 'decimal',
    currency
  } = options;

  try {
    return new Intl.NumberFormat(locale, {
      style,
      currency: style === 'currency' ? currency : undefined,
      minimumFractionDigits,
      maximumFractionDigits,
      useGrouping: showGrouping
    }).format(numValue);
  } catch (error) {
    // Fallback to basic formatting if Intl formatting fails
    return numValue.toString();
  }
}

/**
 * Format a cryptocurrency amount with appropriate precision
 */
export function formatAmount(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  maxDecimals: number = 7,
  options: Omit<NumberFormatOptions, 'maximumFractionDigits'> = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0';
  }

  // Always use the exact maxDecimals provided and let Intl handle trimming
  return formatNumber(numValue, locale, {
    ...options,
    maximumFractionDigits: maxDecimals,
    minimumFractionDigits: 0
  });
}

/**
 * Format a price/rate with appropriate precision
 */
export function formatPrice(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  options: PriceFormatOptions = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0';
  }

  const {
    precision,
    minimumFractionDigits,
    maximumFractionDigits = 6,
    ...restOptions
  } = options;

  // Auto-adjust precision based on value magnitude
  let effectiveMinDecimals = minimumFractionDigits ?? 0;
  let effectiveMaxDecimals = maximumFractionDigits;

  if (precision !== undefined) {
    effectiveMaxDecimals = precision;
    effectiveMinDecimals = precision;
  } else {
    const absValue = Math.abs(numValue);
    if (absValue < 0.01) {
      effectiveMaxDecimals = 6;
      effectiveMinDecimals = 6;
    } else if (absValue < 1) {
      effectiveMaxDecimals = 6;
      effectiveMinDecimals = 6;
    } else if (absValue >= 1000) {
      effectiveMaxDecimals = 0;
      effectiveMinDecimals = 0;
    } else {
      effectiveMaxDecimals = 2;
      effectiveMinDecimals = 2;
    }
  }

  // Special handling for zero values
  if (numValue === 0) {
    effectiveMinDecimals = 0;
    effectiveMaxDecimals = 0;
  }

  return formatNumber(numValue, locale, {
    ...restOptions,
    minimumFractionDigits: effectiveMinDecimals,
    maximumFractionDigits: effectiveMaxDecimals
  });
}

/**
 * Format a percentage value
 */
export function formatPercentage(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  options: Omit<NumberFormatOptions, 'style'> = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0%';
  }

  // Convert decimal to percentage (multiply by 100)
  const percentageValue = numValue * 100;
  
  return formatNumber(percentageValue, locale, {
    ...options,
    style: 'decimal',
    minimumFractionDigits: options.minimumFractionDigits ?? 2,
    maximumFractionDigits: options.maximumFractionDigits ?? 2
  }) + '%';
}

/**
 * Format a currency amount with proper symbol
 */
export function formatCurrency(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  currency: string = 'USD',
  options: Omit<NumberFormatOptions, 'style' | 'currency'> = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0';
  }

  // Special handling for JPY which doesn't use decimal places
  if (currency === 'JPY') {
    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
      ...options
    }).format(numValue);
  }

  return formatNumber(numValue, locale, {
    ...options,
    style: 'currency',
    currency
  });
}

/**
 * Format a balance with compact notation for large numbers
 */
export function formatCompactAmount(
  value: number | string,
  locale: Locale = DEFAULT_LOCALE,
  options: Omit<NumberFormatOptions, 'maximumFractionDigits'> = {}
): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (!Number.isFinite(numValue)) {
    return '0';
  }

  const absValue = Math.abs(numValue);
  
  // Use compact notation for very large numbers
  if (absValue >= 1_000_000) {
    try {
      return new Intl.NumberFormat(locale, {
        notation: 'compact',
        compactDisplay: 'short',
        minimumFractionDigits: 0,
        maximumFractionDigits: 2,
        ...options
      }).format(numValue);
    } catch {
      // Fallback if compact notation not supported
      return formatNumber(numValue, locale, {
        ...options,
        maximumFractionDigits: 0
      });
    }
  }

  return formatAmount(numValue, locale, 7, options);
}

/**
 * Get user's preferred locale from browser
 */
export function getUserLocale(): Locale {
  if (typeof window === 'undefined') {
    return DEFAULT_LOCALE;
  }

  const browserLocale = navigator.language;
  
  // Check if browser locale is directly supported
  if (browserLocale in SUPPORTED_LOCALES) {
    return browserLocale as Locale;
  }

  // Check language part (e.g., 'en' from 'en-US')
  const language = browserLocale.split('-')[0];
  const supportedLocale = Object.keys(SUPPORTED_LOCALES).find(
    locale => locale.startsWith(language)
  ) as Locale | undefined;

  return supportedLocale || DEFAULT_LOCALE;
}

/**
 * Format a rate string like "1 XLM ≈ 0.98 USDC"
 */
export function formatRate(
  fromAmount: number | string,
  fromSymbol: string,
  toAmount: number | string, 
  toSymbol: string,
  locale: Locale = DEFAULT_LOCALE
): string {
  const formattedFrom = formatAmount(fromAmount, locale, 0);
  
  // Always format to 2 decimal places for rates
  const formattedTo = formatNumber(toAmount, locale, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2
  });
  
  return `1 ${fromSymbol} ≈ ${formattedTo} ${toSymbol}`;
}

/**
 * Format a fee amount with appropriate precision
 */
export function formatFee(
  value: number | string,
  symbol: string,
  locale: Locale = DEFAULT_LOCALE,
  options: NumberFormatOptions = {}
): string {
  const formatted = formatAmount(value, locale, 7, options);
  return `${formatted} ${symbol}`;
}
