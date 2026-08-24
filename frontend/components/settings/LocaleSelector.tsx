"use client";

import { useSettings } from '@/components/providers/settings-provider';
import { SUPPORTED_LOCALES, Locale } from '@/lib/formatting';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { useSwapI18n } from '@/lib/swap-i18n';
import type { SwapTranslationKey } from '@/lib/swap-i18n';

export function LocaleSelector() {
  const { settings, updateLocale } = useSettings();
  const currentLocale = settings.locale;
  const { t } = useSwapI18n();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{t('settings.locale.title')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <p className="text-sm text-muted-foreground mb-4">
          {t('settings.locale.description')}
        </p>
        <div className="grid gap-2">
          {Object.entries(SUPPORTED_LOCALES).map(([locale, displayName]) => (
            <Button
              key={locale}
              variant={currentLocale === locale ? "default" : "outline"}
              onClick={() => updateLocale(locale as Locale)}
              className="justify-start h-auto p-3"
            >
              <div className="text-left">
                <div className="font-medium">{displayName}</div>
                <div className="text-xs text-muted-foreground mt-1">
                  {formatExample(locale as Locale, t)}
                </div>
              </div>
            </Button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function formatExample(
  locale: Locale,
  t: (
    key: SwapTranslationKey,
    vars?: Record<string, string | number>,
  ) => string,
): string {
  try {
    const amount = new Intl.NumberFormat(locale, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      useGrouping: true
    }).format(1234.56);
    
    const percentage = new Intl.NumberFormat(locale, {
      style: 'percent',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(0.0123);
    
    return t('settings.locale.example', { amount, percent: percentage });
  } catch {
    return t('settings.locale.example', { amount: '1,234.56', percent: '1.23%' });
  }
}
