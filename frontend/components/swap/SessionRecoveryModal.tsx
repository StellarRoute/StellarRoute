'use client';

import React, { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertCircle, Clock3, RefreshCw } from 'lucide-react';
import type { TradeFormSnapshot } from '@/hooks/useTradeFormStorage';

interface SessionRecoveryModalProps {
  isOpen: boolean;
  reason: 'refresh' | 'wake';
  snapshot: TradeFormSnapshot | null;
  isRecovering?: boolean;
  onRestore: () => Promise<void>;
  onDiscard: () => void;
}

function assetCode(asset: string): string {
  return asset === 'native' ? 'XLM' : asset.split(':')[0];
}

export function SessionRecoveryModal({
  isOpen,
  reason,
  snapshot,
  isRecovering = false,
  onRestore,
  onDiscard,
}: SessionRecoveryModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setIsLoading(isRecovering);
    if (!isRecovering) {
      setError(null);
    }
  }, [isRecovering]);

  const handleRestore = async () => {
    setIsLoading(true);
    setError(null);
    try {
      await onRestore();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to restore session';
      setError(message);
      setIsLoading(false);
    }
  };

  const getTitle = () => {
    switch (reason) {
      case 'wake':
        return 'Resume In-Progress Trade?';
      case 'refresh':
        return 'Restore Previous Trade?';
      default:
        return 'Session Recovery';
    }
  };

  const getDescription = () => {
    switch (reason) {
      case 'wake':
        return 'This tab was idle long enough for the current quote to become unsafe. We\'ll refresh the quote before you can continue trading.';
      case 'refresh':
        return 'A saved draft was found from your last session. We\'ll restore the non-sensitive trade form and fetch a fresh quote before trading.';
      default:
        return 'Restore your previous trading session?';
    }
  };

  const getIcon = () => {
    switch (reason) {
      case 'wake':
        return <Clock3 className="h-6 w-6" />;
      case 'refresh':
        return <RefreshCw className="h-6 w-6" />;
      default:
        return <AlertCircle className="h-6 w-6" />;
    }
  };

  const getActionText = () => {
    if (isLoading) return 'Restoring...';
    return reason === 'refresh' ? 'Restore Session' : 'Refresh Quote';
  };

  return (
    <Dialog open={isOpen} onOpenChange={() => {}}>
      <DialogContent
        showCloseButton={false}
        className="sm:max-w-[440px] rounded-[24px] border-border/40 bg-background/95 p-0 shadow-2xl"
        onPointerDownOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
      >
        <div className="space-y-6 p-7">
          <DialogHeader className="space-y-3">
            <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-primary/10 text-primary">
              {getIcon()}
            </div>
            <DialogTitle className="text-center text-2xl font-bold tracking-tight">
              {getTitle()}
            </DialogTitle>
            <DialogDescription className="text-center text-sm leading-6 text-muted-foreground">
              {getDescription()}
            </DialogDescription>
          </DialogHeader>

          {error && (
            <div className="rounded-2xl border border-destructive/20 bg-destructive/10 p-4 text-sm text-destructive">
              <div className="flex items-center gap-2">
                <AlertCircle className="h-4 w-4 flex-shrink-0" />
                <span>{error}</span>
              </div>
            </div>
          )}

          {snapshot && (
            <div
              data-testid="session-recovery-summary"
              className="space-y-4 rounded-2xl border border-border/30 bg-muted/30 p-5"
            >
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Pair</span>
                <span className="font-semibold">
                  {assetCode(snapshot.fromToken)} to {assetCode(snapshot.toToken)}
                </span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Amount</span>
                <span className="font-semibold">{snapshot.amount || 'Not set'}</span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Slippage</span>
                <span className="font-semibold">{snapshot.slippage}%</span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Deadline</span>
                <span className="font-semibold">{snapshot.deadline} min</span>
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="border-t border-border/20 bg-muted/10 p-6">
          <Button
            variant="outline"
            onClick={onDiscard}
            disabled={isLoading}
            className="h-11 flex-1 rounded-xl font-semibold"
            data-testid="start-fresh-button"
          >
            Start Fresh
          </Button>
          <Button
            onClick={handleRestore}
            disabled={isLoading}
            className="h-11 flex-1 rounded-xl font-semibold gap-2"
            data-testid="restore-session-button"
          >
            {isLoading && <RefreshCw className="h-4 w-4 animate-spin" />}
            {getActionText()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
