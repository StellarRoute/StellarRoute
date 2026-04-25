'use client';

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { AlertCircle, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";

interface SlippageSettingsProps {
  value: number;
  onChange: (val: number) => void;
}

export function SlippageSettings({ value, onChange }: SlippageSettingsProps) {
  const presets = [0.1, 0.5, 1.0];
  
  const handleCustomChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    if (!isNaN(val)) {
      // Clamp between 0.01 and 50
      onChange(Math.max(0.01, Math.min(50, val)));
    } else if (e.target.value === "") {
      // Allow empty input for typing
    }
  };

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
        {presets.map((preset) => (
          <Button
            key={preset}
            variant={value === preset ? "default" : "outline"}
            size="sm"
            onClick={() => onChange(preset)}
            className="flex-1 h-10 font-bold"
          >
            {preset}%
          </Button>
        ))}
        <div className="relative flex-1 min-w-[100px]">
          <Input
            type="number"
            step="0.01"
            min="0.01"
            max="50"
            aria-label="Custom slippage tolerance percentage"
            className={cn(
              "h-10 pr-6 font-bold text-right",
              !presets.includes(value) && "border-primary ring-1 ring-primary/20"
            )}
            placeholder="Custom"
            value={presets.includes(value) ? "" : value}
            onChange={handleCustomChange}
          />
          <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-xs font-bold text-muted-foreground">%</span>
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
