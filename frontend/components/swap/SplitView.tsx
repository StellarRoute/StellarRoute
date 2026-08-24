"use client";

import { ReactNode } from "react";
import { Columns2, LayoutList } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface SplitViewProps {
  /** Left/primary panel (quote form) */
  primary: ReactNode;
  /** Right/secondary panel (route list) */
  secondary: ReactNode;
  /** Whether split-view is active */
  isSplit: boolean;
  /** Toggle callback */
  onToggle: () => void;
  className?: string;
}

/**
 * SplitView layout for the swap page.
 *
 * - Desktop: side-by-side panels when isSplit=true, single column otherwise.
 * - Mobile: always stacked (single column) regardless of isSplit.
 * - Toggle button is keyboard-accessible with aria-pressed.
 */
export function SplitView({
  primary,
  secondary,
  isSplit,
  onToggle,
  className,
}: SplitViewProps) {
  return (
    <div className={cn("w-full", className)}>
      {/* Toggle control */}
      <div className="flex justify-end mb-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onToggle}
          aria-pressed={isSplit}
          aria-label={isSplit ? "Switch to standard layout" : "Switch to split-view layout"}
          className="gap-1.5 text-xs"
        >
          {isSplit ? (
            <>
              <LayoutList className="h-3.5 w-3.5" aria-hidden="true" />
              Standard
            </>
          ) : (
            <>
              <Columns2 className="h-3.5 w-3.5" aria-hidden="true" />
              Split View
            </>
          )}
        </Button>
      </div>

      {/* Layout */}
      <div
        role="region"
        aria-label={isSplit ? "Split view: quote and routes" : "Standard swap view"}
        className={cn(
          "w-full",
          // On mobile always stack; on md+ honour isSplit
          isSplit ? "flex flex-col md:flex-row md:items-start gap-4" : "flex flex-col items-center"
        )}
      >
        {/* Primary panel */}
        <div
          aria-label="Quote panel"
          className={cn(
            "w-full",
            isSplit ? "md:flex-1 md:min-w-0" : "max-w-[480px] mx-auto"
          )}
        >
          {primary}
        </div>

        {/* Secondary panel — always rendered for screen readers, visually hidden when not split on mobile */}
        <div
          aria-label="Route list panel"
          className={cn(
            "w-full",
            isSplit
              ? "md:flex-1 md:min-w-0"
              : "hidden md:hidden" // hidden in standard mode
          )}
        >
          {secondary}
        </div>
      </div>
    </div>
  );
}
