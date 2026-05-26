'use client';

import { useState, useEffect } from 'react';
import { Settings2, RotateCcw } from "lucide-react";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { SlippageSettings } from "./SlippageSettings";
import { DeadlineSettings } from "./DeadlineSettings";
import { ExpertSettings } from "./ExpertSettings";

interface SettingsPanelProps {
  slippage: number;
  onSlippageChange: (value: number) => void;
  deadline: number;
  onDeadlineChange: (value: number) => void;
  expertMode: boolean;
  bypassConfirmation: boolean;
  extendedRouteDetails: boolean;
  onExpertModeChange: (val: boolean) => void;
  onBypassConfirmationChange: (val: boolean) => void;
  onExtendedRouteDetailsChange: (val: boolean) => void;
  onReset: () => void;
}

export function SettingsPanel({
  slippage,
  onSlippageChange,
  deadline,
  onDeadlineChange,
  expertMode,
  bypassConfirmation,
  extendedRouteDetails,
  onExpertModeChange,
  onBypassConfirmationChange,
  onExtendedRouteDetailsChange,
  onReset,
}: SettingsPanelProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isHydrated, setIsHydrated] = useState(false);

  // Safely hydrate drawer open preference from localStorage
  useEffect(() => {
    if (typeof window !== 'undefined') {
      try {
        const storedOpen = localStorage.getItem('stellarroute.settings.drawerOpen') === 'true';
        queueMicrotask(() => {
          setIsOpen(storedOpen);
          setIsHydrated(true);
        });
      } catch (e) {
        console.error('Failed to load settings drawer state', e);
        queueMicrotask(() => {
          setIsHydrated(true);
        });
      }
    }
  }, []);

  const handleOpenChange = (open: boolean) => {
    setIsOpen(open);
    try {
      localStorage.setItem('stellarroute.settings.drawerOpen', String(open));
    } catch (e) {
      console.error('Failed to persist settings drawer state', e);
    }
  };

  return (
    <Sheet open={isHydrated ? isOpen : false} onOpenChange={handleOpenChange}>
      <SheetTrigger asChild>
        <Button 
          variant="ghost" 
          size="icon" 
          className="h-9 w-9 rounded-xl hover:bg-muted/80 hover:text-primary transition-colors min-h-[44px] min-w-[44px]"
        >
          <Settings2 className="h-4.5 w-4.5 text-muted-foreground transition-transform hover:rotate-90 duration-300" />
          <span className="sr-only">Settings</span>
        </Button>
      </SheetTrigger>
      <SheetContent 
        side="right" 
        data-testid="settings-drawer" 
        className="w-full sm:max-w-md p-6 border-l border-border/40 bg-background/95 backdrop-blur-xl shadow-2xl overflow-y-auto flex flex-col gap-6"
      >
        <SheetHeader className="flex flex-row items-center justify-between space-y-0 pb-2 border-b border-border/20">
          <SheetTitle className="text-lg font-bold tracking-tight">Advanced Settings</SheetTitle>
          <Button 
            variant="ghost" 
            size="sm" 
            onClick={onReset}
            className="h-8 text-[11px] font-bold uppercase tracking-widest text-muted-foreground hover:text-primary hover:bg-primary/5 transition-colors gap-1.5 px-3 rounded-full min-h-[36px]"
          >
            <RotateCcw className="h-3 w-3" />
            Reset
          </Button>
        </SheetHeader>

        <div className="space-y-6 flex-1">
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
      </SheetContent>
    </Sheet>
  );
}

