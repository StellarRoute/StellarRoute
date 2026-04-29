'use client';

import { useCallback, useEffect, useState } from 'react';
import { X, AlertCircle, Clock } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface SwapWarning {
  id: string;
  type: 'error' | 'warning';
  title: string;
  message: string;
  timestamp: number; // ISO timestamp
  code?: string; // error code for grouping
  dismissible?: boolean;
}

interface SwapWarningCenterProps {
  onClearAll?: () => void;
  className?: string;
}

const STORAGE_KEY = 'stellarroute_swap_warnings';
const MAX_WARNINGS = 20;

function loadWarnings(): SwapWarning[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as SwapWarning[];
      // Keep only warnings from last 24 hours
      const oneDayMs = 24 * 60 * 60 * 1000;
      const cutoff = Date.now() - oneDayMs;
      return parsed.filter((w) => w.timestamp > cutoff).slice(0, MAX_WARNINGS);
    }
  } catch (e) {
    console.error('Failed to load swap warnings', e);
  }
  return [];
}

function saveWarnings(warnings: SwapWarning[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(warnings));
  } catch (e) {
    console.error('Failed to save swap warnings', e);
  }
}

export function useSwapWarningCenter() {
  const [warnings, setWarnings] = useState<SwapWarning[]>(() => loadWarnings());

  const addWarning = useCallback((warning: Omit<SwapWarning, 'id' | 'timestamp'>) => {
    setWarnings((prev) => {
      const id = `warning_${Date.now()}_${Math.random()}`;
      const newWarning: SwapWarning = {
        ...warning,
        id,
        timestamp: Date.now(),
      };
      const updated = [newWarning, ...prev].slice(0, MAX_WARNINGS);
      saveWarnings(updated);
      return updated;
    });
  }, []);

  const removeWarning = useCallback((id: string) => {
    setWarnings((prev) => {
      const updated = prev.filter((w) => w.id !== id);
      saveWarnings(updated);
      return updated;
    });
  }, []);

  const clearAll = useCallback(() => {
    setWarnings([]);
    saveWarnings([]);
  }, []);

  const clearByType = useCallback((type: 'error' | 'warning') => {
    setWarnings((prev) => {
      const updated = prev.filter((w) => w.type !== type);
      saveWarnings(updated);
      return updated;
    });
  }, []);

  return {
    warnings,
    addWarning,
    removeWarning,
    clearAll,
    clearByType,
  };
}

/**
 * Warning center panel component (collapsed/expandable)
 */
export function SwapWarningCenter({ onClearAll, className }: SwapWarningCenterProps) {
  const { warnings, removeWarning, clearAll } = useSwapWarningCenter();
  const [isOpen, setIsOpen] = useState(false);

  const errorCount = warnings.filter((w) => w.type === 'error').length;
  const warningCount = warnings.filter((w) => w.type === 'warning').length;

  const handleClearAll = useCallback(() => {
    clearAll();
    onClearAll?.();
  }, [clearAll, onClearAll]);

  if (warnings.length === 0) return null;

  return (
    <div className={cn('space-y-2', className)}>
      {/* Collapsed Header (when closed) */}
      {!isOpen && (
        <button
          onClick={() => setIsOpen(true)}
          className="w-full flex items-center justify-between gap-2 px-3 py-2 rounded-lg border border-amber-500/30 bg-amber-500/5 hover:bg-amber-500/10 transition-colors"
        >
          <div className="flex items-center gap-2 text-sm">
            <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
            <span className="font-medium text-amber-700 dark:text-amber-300">
              Recent Swap Issues ({warnings.length})
            </span>
          </div>
          <span className="text-xs text-amber-600 dark:text-amber-400">
            {errorCount > 0 && `${errorCount} error${errorCount !== 1 ? 's' : ''}`}
            {errorCount > 0 && warningCount > 0 && ', '}
            {warningCount > 0 && `${warningCount} warning${warningCount !== 1 ? 's' : ''}`}
          </span>
        </button>
      )}

      {/* Expanded Panel (when open) */}
      {isOpen && (
        <div className="rounded-lg border border-border/40 bg-background/60 backdrop-blur-sm overflow-hidden">
          {/* Header */}
          <div className="flex items-center justify-between gap-2 px-4 py-3 border-b border-border/20">
            <div className="flex items-center gap-2">
              <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400" />
              <span className="text-sm font-semibold">Swap Warning Center</span>
            </div>
            <div className="flex items-center gap-2">
              {warnings.length > 0 && (
                <button
                  onClick={handleClearAll}
                  className="text-xs text-muted-foreground hover:text-foreground transition-colors underline"
                >
                  Clear all
                </button>
              )}
              <button
                onClick={() => setIsOpen(false)}
                className="rounded p-1 hover:bg-muted transition-colors"
                aria-label="Close warning center"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
          </div>

          {/* Warnings List */}
          <div className="max-h-[400px] overflow-y-auto divide-y divide-border/20">
            {warnings.length === 0 ? (
              <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                No warnings to display
              </p>
            ) : (
              warnings.map((warning) => (
                <div
                  key={warning.id}
                  className={cn(
                    'px-4 py-3 text-sm',
                    warning.type === 'error'
                      ? 'bg-destructive/5 hover:bg-destructive/10'
                      : 'bg-amber-500/5 hover:bg-amber-500/10'
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span
                          className={cn(
                            'inline-block text-[10px] font-bold uppercase tracking-wide px-1.5 py-0.5 rounded',
                            warning.type === 'error'
                              ? 'bg-destructive text-destructive-foreground'
                              : 'bg-amber-600 text-white'
                          )}
                        >
                          {warning.type}
                        </span>
                        <span className="text-[10px] text-muted-foreground flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          {new Date(warning.timestamp).toLocaleTimeString()}
                        </span>
                      </div>
                      <p className="font-medium leading-snug">{warning.title}</p>
                      <p className="mt-0.5 text-xs text-muted-foreground leading-relaxed">
                        {warning.message}
                      </p>
                      {warning.code && (
                        <p className="mt-1 text-[10px] font-mono text-muted-foreground">
                          Code: {warning.code}
                        </p>
                      )}
                    </div>
                    {warning.dismissible !== false && (
                      <button
                        onClick={() => removeWarning(warning.id)}
                        className="shrink-0 rounded p-1 hover:bg-muted transition-colors"
                        aria-label={`Dismiss: ${warning.title}`}
                      >
                        <X className="h-3.5 w-3.5 text-muted-foreground" />
                      </button>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Inline alert component for displaying warnings without opening the center
 */
export function SwapWarningAlert({
  warning,
  onDismiss,
}: {
  warning: SwapWarning;
  onDismiss?: () => void;
}) {
  return (
    <div
      className={cn(
        'flex gap-2 p-3 rounded-lg border text-xs',
        warning.type === 'error'
          ? 'bg-destructive/10 border-destructive/20 text-destructive'
          : 'bg-amber-500/10 border-amber-500/20 text-amber-700 dark:text-amber-300'
      )}
    >
      <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
      <div className="flex-1 min-w-0">
        <p className="font-medium">{warning.title}</p>
        <p className="text-[11px] opacity-80">{warning.message}</p>
      </div>
      {(warning.dismissible !== false || onDismiss) && (
        <button
          onClick={onDismiss}
          className="shrink-0 rounded hover:bg-current/10 transition-colors"
          aria-label="Dismiss warning"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}
