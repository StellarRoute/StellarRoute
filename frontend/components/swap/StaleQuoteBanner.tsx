'use client';

import { RefreshCw, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface StaleQuoteBannerProps {
  isStale: boolean;
  onRefresh: () => void;
  isLoading: boolean;
  className?: string;
}

export function StaleQuoteBanner({
  isStale,
  onRefresh,
  isLoading,
  className,
}: StaleQuoteBannerProps) {
  if (!isStale) return null;

  return (
    <div
      className={cn(
        "relative overflow-hidden group mb-4",
        "bg-amber-500/10 border border-amber-500/20 backdrop-blur-md",
        "rounded-2xl p-4 animate-in slide-in-from-top-4 duration-500",
        className
      )}
      data-testid="stale-indicator"
    >
      {/* Animated highlight effect */}
      <div className="absolute inset-0 bg-gradient-to-r from-transparent via-amber-500/5 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-1000 ease-in-out" />
      
      <div className="relative flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="flex-shrink-0 h-10 w-10 rounded-xl bg-amber-500/20 flex items-center justify-center">
            <AlertTriangle className="h-5 w-5 text-amber-500" />
          </div>
          <div className="flex flex-col">
            <span className="text-sm font-bold text-amber-500">
              Quote Expired
            </span>
            <span className="text-xs text-amber-500/80 font-medium">
              Market prices have shifted. Refresh to get the latest quote.
            </span>
          </div>
        </div>
        
        <Button
          size="sm"
          onClick={onRefresh}
          disabled={isLoading}
          className="bg-amber-500 hover:bg-amber-600 text-white font-bold rounded-xl shadow-lg shadow-amber-500/20 transition-all active:scale-95"
        >
          <RefreshCw className={cn("mr-2 h-4 w-4", isLoading && "animate-spin")} />
          Refresh
        </Button>
      </div>
    </div>
  );
}
