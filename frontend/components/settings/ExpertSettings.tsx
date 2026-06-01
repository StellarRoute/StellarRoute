'use client';

import { Checkbox } from '@/components/ui/checkbox';
import { AlertTriangle, ShieldAlert, Zap, Compass } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ExpertSettingsProps {
  expertMode: boolean;
  bypassConfirmation: boolean;
  extendedRouteDetails: boolean;
  onExpertModeChange: (val: boolean) => void;
  onBypassConfirmationChange: (val: boolean) => void;
  onExtendedRouteDetailsChange: (val: boolean) => void;
}

export function ExpertSettings({
  expertMode,
  bypassConfirmation,
  extendedRouteDetails,
  onExpertModeChange,
  onBypassConfirmationChange,
  onExtendedRouteDetailsChange,
}: ExpertSettingsProps) {

  return (
    <div className="space-y-4 pt-4 border-t border-border/20">
      {/* Header & Main Toggle */}
      <div className="flex items-center justify-between min-h-[44px]">
        <div className="flex items-center gap-2">
          <ShieldAlert className={cn(
            "h-4 w-4 transition-colors duration-300",
            expertMode ? "text-amber-500 animate-pulse" : "text-muted-foreground"
          )} />
          <span className="text-sm font-semibold tracking-tight">Expert Mode</span>
        </div>
        <button
          role="switch"
          aria-checked={expertMode}
          aria-label="Toggle Expert Mode"
          onClick={() => onExpertModeChange(!expertMode)}
          className={cn(
            "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 min-h-[44px] min-w-[44px] items-center",
            expertMode ? "bg-amber-500 shadow-lg shadow-amber-500/20" : "bg-muted"
          )}
        >
          <span
            className={cn(
              "pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform duration-300",
              expertMode ? "translate-x-5" : "translate-x-0"
            )}
          />
        </button>
      </div>

      {/* Warning block if expert mode is active */}
      {expertMode && (
        <div className="space-y-4 animate-in fade-in slide-in-from-top-2 duration-300">
          <div className="flex items-start gap-2.5 p-3.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-[11px] text-amber-600 dark:text-amber-400 font-medium leading-relaxed">
            <AlertTriangle className="h-4 w-4 flex-shrink-0 mt-0.5" />
            <p>
              Expert Mode enables highly custom values and features. Be careful: high slippage limits can result in bad execution prices or frontrunning.
            </p>
          </div>

          {/* Sub options */}
          <div className="space-y-3 pl-2">
            {/* Bypass Confirmation Option */}
            <label className="flex items-start gap-3 p-2 rounded-xl hover:bg-muted/30 cursor-pointer transition-colors group min-h-[44px]">
              <Checkbox
                checked={bypassConfirmation}
                onCheckedChange={(checked) => onBypassConfirmationChange(!!checked)}
                className="mt-1 border-border/40 data-[state=checked]:bg-amber-500 data-[state=checked]:border-amber-500"
              />
              <div className="space-y-0.5">
                <div className="flex items-center gap-1.5 text-xs font-bold tracking-tight text-foreground group-hover:text-amber-500 transition-colors">
                  <Zap className="h-3.5 w-3.5" />
                  Bypass Confirmation Modal
                </div>
                <p className="text-[10px] text-muted-foreground leading-normal">
                  Execute transactions instantly with a single click. Useful for fast-moving markets.
                </p>
              </div>
            </label>

            {/* Extended Route Diagnostics */}
            <label className="flex items-start gap-3 p-2 rounded-xl hover:bg-muted/30 cursor-pointer transition-colors group min-h-[44px]">
              <Checkbox
                checked={extendedRouteDetails}
                onCheckedChange={(checked) => onExtendedRouteDetailsChange(!!checked)}
                className="mt-1 border-border/40 data-[state=checked]:bg-amber-500 data-[state=checked]:border-amber-500"
              />
              <div className="space-y-0.5">
                <div className="flex items-center gap-1.5 text-xs font-bold tracking-tight text-foreground group-hover:text-amber-500 transition-colors">
                  <Compass className="h-3.5 w-3.5" />
                  Extended Route Diagnostics
                </div>
                <p className="text-[10px] text-muted-foreground leading-normal">
                  Show raw liquidity pools, pool reserves, and exact multi-hop route metrics.
                </p>
              </div>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}
