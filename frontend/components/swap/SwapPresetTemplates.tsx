"use client";

import React from "react";
import { History, Zap } from "lucide-react";
import { cn } from "@/lib/utils";
import { SwapPreset } from "@/lib/presets";
import { useSwapPresetTemplates } from "@/hooks/useSwapPresetTemplates";

export interface SwapPresetTemplatesProps {
  onSelect: (base: string, quote: string) => void;
  selectedBase?: string;
  selectedQuote?: string;
  className?: string;
}

export function SwapPresetTemplates({
  onSelect,
  selectedBase,
  selectedQuote,
  className,
}: SwapPresetTemplatesProps) {
  const { templates, recentTemplates, addRecentTemplate } = useSwapPresetTemplates();

  const handleSelect = (preset: SwapPreset) => {
    onSelect(preset.baseAsset, preset.quoteAsset);
    addRecentTemplate(preset);
  };

  if (templates.length === 0) return null;

  return (
    <div className={cn("flex flex-col gap-3 pb-2", className)}>
      <div className="flex items-center justify-between px-1">
        <div className="flex items-center gap-1.5">
          <Zap className="size-3.5 text-primary animate-pulse" />
          <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">
            Quick Pairs
          </span>
        </div>
        {recentTemplates.length > 0 && (
          <span className="text-[10px] font-medium text-primary/60 flex items-center gap-1">
            <History className="size-2.5" />
            Recent
          </span>
        )}
      </div>
      
      <div className="flex flex-wrap gap-2.5">
        {templates.map((preset, index) => {
          const isSelected = 
            selectedBase === preset.baseAsset && 
            selectedQuote === preset.quoteAsset;
          
          const isRecent = recentTemplates.some(r => r.id === preset.id);

          return (
            <button
              key={preset.id}
              type="button"
              onClick={() => handleSelect(preset)}
              style={{ animationDelay: `${index * 50}ms` }}
              className={cn(
                "group relative flex items-center gap-2 rounded-xl px-4 py-2 text-xs font-semibold transition-all duration-500",
                "border backdrop-blur-xl shadow-sm animate-in fade-in slide-in-from-left-4",
                isSelected
                  ? "bg-primary/15 border-primary/50 text-primary ring-1 ring-primary/30 shadow-[0_0_15px_rgba(var(--primary),0.1)] scale-105"
                  : "bg-background/20 border-white/5 text-muted-foreground hover:bg-white/5 hover:border-primary/20 hover:text-foreground hover:scale-105 active:scale-95"
              )}
            >
              {isRecent && (
                <History className="size-3 opacity-60 group-hover:opacity-100 group-hover:rotate-[-45deg] transition-all duration-300" />
              )}
              {preset.label}
              
              {/* Premium Glow Effect */}
              <div className="absolute inset-0 -z-10 rounded-xl opacity-0 group-hover:opacity-100 transition-opacity duration-500 bg-gradient-to-br from-primary/10 via-transparent to-blue-500/10 blur-xl" />
              
              {/* Subtle Border Light */}
              <div className={cn(
                "absolute inset-px rounded-[11px] opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none",
                "bg-gradient-to-br from-white/10 to-transparent"
              )} />
            </button>
          );
        })}
      </div>
    </div>
  );
}
