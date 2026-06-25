'use client';

import { Settings2, RotateCcw } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { SlippageSettings } from "./SlippageSettings";
import { DeadlineSettings } from "./DeadlineSettings";
import { ExpertSettings } from "./ExpertSettings";
import { useExpertSettings } from "@/hooks/useExpertSettings";
import { useTradeFormStorage } from "@/hooks/useTradeFormStorage";
import { useSettings } from "@/components/providers/settings-provider";
import { useSwapI18n } from "@/lib/swap-i18n";

interface SettingsPanelProps {
  expertSettings?: {
    expertMode: boolean;
    bypassConfirmation: boolean;
    extendedRouteDetails: boolean;
    updateExpertMode: (val: boolean) => void;
    updateBypassConfirmation: (val: boolean) => void;
    updateExtendedRouteDetails: (val: boolean) => void;
  };
}

export function SettingsPanel({ expertSettings: customExpertSettings }: SettingsPanelProps = {}) {
  const localExpertSettings = useExpertSettings();
  const expertSettings = customExpertSettings ?? localExpertSettings;
  const { deadline, setDeadline, reset: resetForm } = useTradeFormStorage();
  const { resetSettings } = useSettings();
  const { t } = useSwapI18n();

  const handleReset = () => {
    resetForm();
    resetSettings();
    expertSettings.updateExpertMode(false);
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button 
          variant="ghost" 
          size="icon" 
          className="h-10 w-10 rounded-xl hover:bg-muted/80 hover:text-primary transition-colors"
        >
          <Settings2 className="h-5 w-5 text-muted-foreground transition-transform hover:rotate-90 duration-300" />
          <span className="sr-only">{t('swap.settings.buttonLabel')}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" data-testid="settings-panel" className="w-[320px] p-6 rounded-[24px] shadow-2xl border-border/40 bg-background/95 backdrop-blur-xl animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-bold tracking-tight">{t('settings.panel.title')}</h3>
          <Button 
            variant="ghost" 
            size="sm" 
            onClick={handleReset}
            className="h-8 text-[11px] font-bold uppercase tracking-widest text-muted-foreground hover:text-primary hover:bg-primary/5 transition-colors gap-1.5 px-3 rounded-full"
          >
            <RotateCcw className="h-3 w-3" />
            {t('settings.panel.reset')}
          </Button>
        </div>

        <div className="space-y-6">
          <SlippageSettings />
          <DeadlineSettings value={deadline} onChange={setDeadline} />
          <ExpertSettings
            expertMode={expertSettings.expertMode}
            bypassConfirmation={expertSettings.bypassConfirmation}
            extendedRouteDetails={expertSettings.extendedRouteDetails}
            onExpertModeChange={expertSettings.updateExpertMode}
            onBypassConfirmationChange={expertSettings.updateBypassConfirmation}
            onExtendedRouteDetailsChange={expertSettings.updateExtendedRouteDetails}
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}
