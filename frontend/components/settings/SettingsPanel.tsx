'use client';

import { RotateCcw, Settings2 } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { useSwapI18n } from '@/lib/swap-i18n';
import { DeadlineSettings } from './DeadlineSettings';
import { ExpertSettings } from './ExpertSettings';
import { SlippageSettings } from './SlippageSettings';

export interface SettingsPanelProps {
  slippage: number;
  deadline: number;
  expertMode: boolean;
  bypassConfirmation: boolean;
  extendedRouteDetails: boolean;
  onSlippageChange: (value: number) => void;
  onDeadlineChange: (value: number) => void;
  onExpertModeChange: (value: boolean) => void;
  onBypassConfirmationChange: (value: boolean) => void;
  onExtendedRouteDetailsChange: (value: boolean) => void;
  onReset: () => void;
}

export function SettingsPanel({
  slippage,
  deadline,
  expertMode,
  bypassConfirmation,
  extendedRouteDetails,
  onSlippageChange,
  onDeadlineChange,
  onExpertModeChange,
  onBypassConfirmationChange,
  onExtendedRouteDetailsChange,
  onReset,
}: SettingsPanelProps) {
  const { t } = useSwapI18n();
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-10 w-10 rounded-xl transition-colors hover:bg-muted/80 hover:text-primary"
        >
          <Settings2 className="h-5 w-5 text-muted-foreground transition-transform duration-300 hover:rotate-90" />
          <span className="sr-only">{t('swap.settings.buttonLabel')}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        data-testid="settings-panel"
        className="w-[min(360px,calc(100vw-2rem))] rounded-[24px] border-border/40 bg-background/95 p-6 shadow-2xl backdrop-blur-xl"
      >
        <div className="mb-6 flex items-center justify-between">
          <h3 className="text-lg font-bold tracking-tight">
            Advanced Settings
          </h3>
          <Button
            variant="ghost"
            size="sm"
            onClick={onReset}
            className="h-8 gap-1.5 rounded-full px-3 text-[11px] font-bold uppercase tracking-widest text-muted-foreground transition-colors hover:bg-primary/5 hover:text-primary"
          >
            <RotateCcw className="h-3 w-3" />
            {t('settings.panel.reset')}
          </Button>
        </div>

        <div className="space-y-6">
          <SlippageSettings value={slippage} onChange={onSlippageChange} />
          <DeadlineSettings value={deadline} onChange={onDeadlineChange} />
          <ExpertSettings
            expertMode={expertMode}
            bypassConfirmation={bypassConfirmation}
            extendedRouteDetails={extendedRouteDetails}
            onExpertModeChange={onExpertModeChange}
            onBypassConfirmationChange={onBypassConfirmationChange}
            onExtendedRouteDetailsChange={onExtendedRouteDetailsChange}
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}
