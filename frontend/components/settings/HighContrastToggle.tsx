'use client';

import { useSettings } from '@/components/providers/settings-provider';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';

export function HighContrastToggle() {
  const { settings, updateHighContrast } = useSettings();

  return (
    <div className="flex items-center justify-between gap-4">
      <div className="space-y-0.5">
        <Label htmlFor="high-contrast-toggle" className="text-sm font-medium cursor-pointer">
          High Contrast Mode
        </Label>
        <p className="text-xs text-muted-foreground">
          Increases color contrast for improved readability and accessibility.
        </p>
      </div>
      <Switch
        id="high-contrast-toggle"
        checked={settings.highContrast}
        onCheckedChange={updateHighContrast}
        aria-label="Toggle high contrast mode"
      />
    </div>
  );
}
