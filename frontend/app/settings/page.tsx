'use client';

import { useState, useEffect } from 'react';
import { useSettings } from '@/components/providers/settings-provider';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ThemeSetting } from '@/types/settings';
import { toast } from 'sonner';
import { LocaleSelector } from '@/components/settings/LocaleSelector';

export default function SettingsPage() {
  const { settings, updateSlippage, updateTheme, resetSettings } = useSettings();
  const [localSlippage, setLocalSlippage] = useState(settings.slippageTolerance.toString());

  useEffect(() => {
    setLocalSlippage(settings.slippageTolerance.toString());
  }, [settings.slippageTolerance]);

  const handleSlippageChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setLocalSlippage(e.target.value);
  };

  const handleSlippageBlur = () => {
    const value = parseFloat(localSlippage);
    if (!isNaN(value) && value >= 0 && value <= 50) {
      updateSlippage(value);
    } else {
      setLocalSlippage(settings.slippageTolerance.toString());
      toast.error('Slippage must be between 0 and 50%');
    }
  };

  const handleReset = () => {
    resetSettings();
    toast.success('Settings reset to defaults');
  };

  return (
    <div className="container mx-auto py-10 px-4 max-w-2xl">
      <h1 className="text-3xl font-bold mb-6">Settings</h1>
      
      <div className="space-y-6">
        <LocaleSelector />

        <Card>
          <CardHeader>
            <CardTitle>Trade Settings</CardTitle>
            <CardDescription>
              Configure your default trading parameters.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Default Slippage Tolerance (%)</label>
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
                  Typical: 0.5% - 1.0%
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Appearance</CardTitle>
            <CardDescription>
              Customize how StellarRoute looks on your device.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Theme</label>
              <Select
                value={settings.theme}
                onValueChange={(value) => updateTheme(value as ThemeSetting)}
              >
                <SelectTrigger className="w-[180px]">
                  <SelectValue placeholder="Select theme" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">Light</SelectItem>
                  <SelectItem value="dark">Dark</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>

        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle className="text-destructive">Reset Settings</CardTitle>
            <CardDescription>
              Revert all settings to their original factory defaults.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="destructive" onClick={handleReset}>
              Reset to Defaults
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
