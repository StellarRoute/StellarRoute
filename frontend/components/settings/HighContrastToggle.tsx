'use client';

import { useEffect } from 'react';
import { useSettings } from '@/components/providers/settings-provider';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { useSwapI18n } from '@/lib/swap-i18n';

export function HighContrastToggle() {
  const { settings, updateHighContrast } = useSettings();
  const { t } = useSwapI18n();

  useEffect(() => {
    document.documentElement.classList.toggle('high-contrast', settings.highContrast);
  }, [settings.highContrast]);

  return (
    <div className="flex items-center justify-between gap-4">
      <div className="space-y-0.5">
        <Label htmlFor="high-contrast-toggle" className="text-sm font-medium cursor-pointer">
          {t('settings.highContrast.label')}
        </Label>
        <p className="text-xs text-muted-foreground">
          {t('settings.highContrast.description')}
        </p>
      </div>
      <Switch
        id="high-contrast-toggle"
        checked={settings.highContrast}
        onCheckedChange={updateHighContrast}
        aria-label={`Toggle ${t('settings.highContrast.label')}`}
      />
    </div>
  );
}
