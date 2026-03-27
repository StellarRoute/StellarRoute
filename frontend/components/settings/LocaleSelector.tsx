"use client";

import { useSettings } from '@/components/providers/settings-provider';
import { SUPPORTED_LOCALES, Locale } from '@/lib/formatting';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

export function LocaleSelector() {
  const { settings, updateLocale } = useSettings();
  const currentLocale = settings.locale;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Language & Region</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <p className="text-sm text-muted-foreground mb-4">
          Choose your preferred language and number formatting. This affects how amounts, prices, and other numbers are displayed.
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
                  Example: {formatExample(locale as Locale)}
                </div>
              </div>
            </Button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function formatExample(locale: Locale): string {
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
    
    return `${amount} • ${percentage}`;
  } catch {
    return '1,234.56 • 1.23%';
  }
}
