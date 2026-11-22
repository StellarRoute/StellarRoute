'use client';

import { useState } from 'react';
import { useSettings } from '@/components/providers/settings-provider';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ThemeSetting } from '@/types/settings';
import { toast } from 'sonner';
import { LocaleSelector } from '@/components/settings/LocaleSelector';
import { AccentColorPicker } from '@/components/settings/AccentColorPicker';
import { FontScaleControl } from '@/components/settings/FontScaleControl';
import { HighContrastToggle } from '@/components/settings/HighContrastToggle';
import { BrowserNotificationSettings } from '@/components/settings/BrowserNotificationSettings';
import { ThemePreview } from '@/components/settings/ThemePreview';
import { useBrowserNotifications } from '@/hooks/useBrowserNotifications';
import { useFeatureFlag } from '@/hooks/useFeatureFlag';
import { useSwapI18n } from '@/lib/swap-i18n';

export function SettingsPageClient() {
  const { settings, updateSlippage, updateTheme, resetSettings } = useSettings();
  const [localSlippage, setLocalSlippage] = useState(settings.slippageTolerance.toString());
  const [pendingTheme, setPendingTheme] = useState<ThemeSetting | null>(null);
  const { enabled: themePreviewEnabled } = useFeatureFlag('settings_theme_preview');
  const {
    browserNotifications,
    permissionState,
    isDisabled,
    enableNotifications,
    disableNotifications,
  } = useBrowserNotifications();
  const { t } = useSwapI18n();

  const handleSlippageChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setLocalSlippage(e.target.value);
  };

  const handleSlippageBlur = () => {
    const value = parseFloat(localSlippage);
    if (!isNaN(value) && value >= 0 && value <= 50) {
      updateSlippage(value);
    } else {
      setLocalSlippage(settings.slippageTolerance.toString());
      toast.error(t('settings.slippage.error'));
    }
  };

  const handleThemeChange = (value: string) => {
    const theme = value as ThemeSetting;
    if (themePreviewEnabled) {
      setPendingTheme(theme);
    } else {
      updateTheme(theme);
    }
  };

  const handleApplyTheme = () => {
    if (pendingTheme) {
      updateTheme(pendingTheme);
      setPendingTheme(null);
      toast.success(t('settings.theme.applied'));
    }
  };

  const handleReset = () => {
    resetSettings();
    setPendingTheme(null);
    toast.success(t('settings.reset.success'));
  };

  return (
    <div className="container mx-auto py-10 px-4 max-w-2xl">
      <h1 className="text-3xl font-bold mb-6">{t('settings.page.title')}</h1>

      <div className="space-y-6">
        <LocaleSelector />

        <Card>
          <CardHeader>
            <CardTitle>{t('settings.trade.title')}</CardTitle>
            <CardDescription>
              {t('settings.trade.description')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('settings.slippage.label')}</label>
              <div className="flex items-center gap-4">
                <Input
                  type="number"
                  step="0.1"
                  min="0"
                  max="50"
                  value={localSlippage}
                  onChange={handleSlippageChange}
                  onBlur={handleSlippageBlur}
                  className="max-w-[150px]"
                />
                <span className="text-sm text-muted-foreground">
                  {t('settings.slippage.typical')}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('settings.appearance.title')}</CardTitle>
            <CardDescription>
              {t('settings.appearance.description')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Theme selector */}
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('settings.theme.label')}</label>
              <Select
                value={pendingTheme ?? settings.theme}
                onValueChange={handleThemeChange}
              >
                <SelectTrigger className="w-[180px]">
                  <SelectValue placeholder={t('settings.theme.placeholder')} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">{t('settings.theme.light')}</SelectItem>
                  <SelectItem value="dark">{t('settings.theme.dark')}</SelectItem>
                  <SelectItem value="system">{t('settings.theme.system')}</SelectItem>
                </SelectContent>
              </Select>
              {themePreviewEnabled && pendingTheme && (
                <div className="space-y-2">
                  <ThemePreview previewTheme={pendingTheme} />
                  <Button
                    size="sm"
                    onClick={handleApplyTheme}
                    data-testid="theme-preview-apply"
                  >
                    Apply theme
                  </Button>
                </div>
              )}
            </div>

            {/* Accent colour picker — issue #521 */}
            <AccentColorPicker />
          </CardContent>
        </Card>

        {/* Font scale control and high contrast — issues #522, #788 */}
        <Card>
          <CardHeader>
            <CardTitle>{t('settings.accessibility.title')}</CardTitle>
            <CardDescription>
              {t('settings.accessibility.description')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <FontScaleControl />
            <HighContrastToggle />
          </CardContent>
        </Card>

        {/* Browser notifications — issue #789 */}
        <Card>
          <CardHeader>
            <CardTitle>{t('settings.notifications.title')}</CardTitle>
            <CardDescription>
              {t('settings.notifications.description')}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <BrowserNotificationSettings
              browserNotifications={browserNotifications}
              permissionState={permissionState}
              isDisabled={isDisabled}
              onEnable={enableNotifications}
              onDisable={disableNotifications}
            />
          </CardContent>
        </Card>

        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle className="text-destructive">{t('settings.reset.title')}</CardTitle>
            <CardDescription>
              {t('settings.reset.description')}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="destructive" onClick={handleReset}>
              {t('settings.reset.button')}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
