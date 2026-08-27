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
import { useBrowserNotifications } from '@/hooks/useBrowserNotifications';
import { useSwapI18n } from '@/lib/swap-i18n';

export function SettingsPageClient() {
  const { settings, updateSlippage, updateTheme, resetSettings } = useSettings();
  const [localSlippage, setLocalSlippage] = useState(settings.slippageTolerance.toString());
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

  const handleReset = () => {
    resetSettings();
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
                value={settings.theme}
                onValueChange={(value) => updateTheme(value as ThemeSetting)}
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

        {/* Developer & Feature Flag Help — issue #1297 */}
        <Card data-testid="settings-feature-flags-help">
          <CardHeader>
            <CardTitle>Feature Flags &amp; URL Reference</CardTitle>
            <CardDescription>
              StellarRoute keeps a clean header without extra navigation chrome. You can access this page directly at <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-foreground">/settings</code>.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground leading-relaxed">
              Experimental UI capabilities are controlled via client feature flags configured via environment variables or remote flags configuration:
            </p>
            <div className="space-y-3">
              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">routes_beta</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_ROUTES_BETA</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables the experimental routing inspector and alternative path explorer.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">batch_swaps</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_BATCH_SWAPS</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables multi-operation atomic batch swap execution capabilities.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">swap_ui_v2</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_SWAP_UI_V2</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables the wide Stellar-centered cross-chain corridor and route deck view.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">transaction_history</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables the user transaction history tab in the swap interface.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">advanced_slippage</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables customized sub-basis point and dynamic slippage controls.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">analytics</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FEATURE_ANALYTICS</span>
                </div>
                <p className="text-xs text-muted-foreground">Enables aggregated liquidity and volume metrics analytics dashboard.</p>
              </div>

              <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs font-semibold text-foreground">real_xdr</span>
                  <span className="text-[11px] font-mono text-muted-foreground">NEXT_PUBLIC_FLAG_REAL_XDR</span>
                </div>
                <p className="text-xs text-muted-foreground">Security-pinned default (on): API prepare → Freighter wallet sign → API submit path.</p>
              </div>
            </div>
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
