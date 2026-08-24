'use client';

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useSwapI18n } from "@/lib/swap-i18n";

interface DeadlineSettingsProps {
  value: number;
  onChange: (val: number) => void;
}

export function DeadlineSettings({ value, onChange }: DeadlineSettingsProps) {
  const { t } = useSwapI18n();
  const presets = [10, 30, 60];
  
  const handleCustomChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value);
    if (!isNaN(val) && val > 0) {
      onChange(Math.min(1440, val)); // Max 24h
    }
  };

  return (
    <div className="space-y-4 pt-4 border-t border-border/20">
      <div className="flex items-center justify-between">
        <span className="text-sm font-semibold tracking-tight">{t('settings.deadline.label')}</span>
        <span className="text-xs font-bold text-muted-foreground px-2 py-0.5 rounded-full bg-muted/50">
          {value} {t('settings.deadline.min')}
        </span>
      </div>

      <div className="flex flex-wrap gap-2">
        {presets.map((preset) => (
          <Button
            key={preset}
            variant={value === preset ? "default" : "outline"}
            size="sm"
            onClick={() => onChange(preset)}
            className="flex-1 h-10 font-bold"
          >
            {preset === 60 ? t('settings.deadline.preset1h') : (preset === 10 ? t('settings.deadline.preset10m') : t('settings.deadline.preset30m'))}
          </Button>
        ))}
        <div className="relative flex-1 min-w-[100px]">
          <Input
            type="number"
            min="1"
            max="1440"
            className={cn(
              "h-10 pr-10 font-bold text-right",
              !presets.includes(value) && "border-primary ring-1 ring-primary/20"
            )}
            placeholder={t('settings.deadline.custom')}
            value={presets.includes(value) ? "" : value}
            onChange={handleCustomChange}
          />
          <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs font-bold text-muted-foreground">{t('settings.deadline.min')}</span>
        </div>
      </div>
      <p className="text-[10px] text-muted-foreground/60 italic leading-tight">
        {t('settings.deadline.description')}
      </p>
    </div>
  );
}
