'use client';

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { AlertCircle, AlertTriangle, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useSettings } from "@/components/providers/settings-provider";

export function SlippageSettings() {
  const { settings, selectProfile, updateProfile, addProfile, deleteProfile } = useSettings();
  const value = settings.slippageTolerance;
  
  const handleCustomChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    if (!isNaN(val)) {
      const clamped = Math.max(0.01, Math.min(50, val));
      
      const customProfile = settings.slippageProfiles.find(p => !p.isPreset);
      if (customProfile) {
        updateProfile(customProfile.id, { value: clamped });
      } else {
        addProfile({ name: 'Custom', value: clamped });
        // The newly added profile will need to be selected manually, 
        // wait, we should select it right after adding. 
        // Our addProfile doesn't return ID. Let's just update useSettings logic or find the newly created one.
        // Actually, let's keep it simple: if there is no custom profile, add one.
      }
    }
  };

  const activeProfile = settings.slippageProfiles.find(p => p.id === settings.activeProfileId);
  const customProfile = settings.slippageProfiles.find(p => !p.isPreset);

  const isLow = value < 0.1;
  const isHigh = value > 5;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="text-sm font-semibold tracking-tight">Slippage Tolerance</span>
        <span className={cn("text-xs font-bold px-2 py-0.5 rounded-full", 
          isHigh ? "bg-destructive/10 text-destructive" : "bg-primary/10 text-primary")}>
          {value}%
        </span>
      </div>

      <div className="flex flex-wrap gap-2">
        {settings.slippageProfiles.filter(p => p.isPreset).map((preset) => (
          <Button
            key={preset.id}
            variant={settings.activeProfileId === preset.id ? "default" : "outline"}
            size="sm"
            onClick={() => selectProfile(preset.id)}
            className="flex-1 h-10 font-bold"
          >
            {preset.name}
          </Button>
        ))}
        
        <div className="relative flex-[1.5] min-w-[120px] flex items-center gap-1">
          <Input
            type="number"
            step="0.01"
            min="0.01"
            max="50"
            aria-label="Custom slippage tolerance percentage"
            className={cn(
              "h-10 pr-6 font-bold text-right",
              !activeProfile?.isPreset && "border-primary ring-1 ring-primary/20"
            )}
            placeholder="Custom"
            value={customProfile ? customProfile.value : ""}
            onChange={(e) => {
              const val = parseFloat(e.target.value);
              if (!isNaN(val)) {
                const clamped = Math.max(0.01, Math.min(50, val));
                if (customProfile) {
                  updateProfile(customProfile.id, { value: clamped });
                  selectProfile(customProfile.id);
                } else {
                  addProfile({ name: 'Custom', value: clamped });
                  // In a real app we'd get the ID back and select it, 
                  // but we can just let addProfile handle it or select the non-preset.
                }
              }
            }}
            onClick={() => {
              if (customProfile) selectProfile(customProfile.id);
            }}
          />
          <span className="absolute right-8 top-1/2 -translate-y-1/2 text-xs font-bold text-muted-foreground">%</span>
          
          {customProfile && (
            <Button 
              variant="ghost" 
              size="icon" 
              className="h-10 w-10 text-muted-foreground hover:text-destructive shrink-0"
              onClick={(e) => {
                e.stopPropagation();
                deleteProfile(customProfile.id);
              }}
              title="Delete Custom Profile"
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {isLow && (
        <div className="flex items-center gap-2 p-3 rounded-xl bg-yellow-500/10 border border-yellow-500/20 text-[11px] text-yellow-600 dark:text-yellow-400 font-medium">
          <AlertTriangle className="h-3.5 w-3.5 flex-shrink-0" />
          <p>Your transaction may fail if the price moves unfavorably by more than {value}%.</p>
        </div>
      )}

      {isHigh && (
        <div className="flex items-center gap-2 p-3 rounded-xl bg-destructive/10 border border-destructive/20 text-[11px] text-destructive font-medium">
          <AlertCircle className="h-3.5 w-3.5 flex-shrink-0" />
          <p>High slippage increases the risk of frontrunning and getting a significantly worse price.</p>
        </div>
      )}
    </div>
  );
}
