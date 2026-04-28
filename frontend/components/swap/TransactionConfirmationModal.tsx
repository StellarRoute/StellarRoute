'use client';

import { useEffect, useRef } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import {
  ArrowRightLeft,
  CheckCircle2,
  Clock,
  Loader2,
  XCircle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TransactionStatus } from '@/types/transaction';
import type { TradeParams } from '@/hooks/useTransactionLifecycle';

export interface TransactionConfirmationModalProps {
  isOpen: boolean;
  status: TransactionStatus | 'review';
  txHash?: string;
  errorMessage?: string;
  tradeParams?: TradeParams;
  onConfirm: () => void;
  onCancel: () => void;
  onTryAgain: () => void;
  onResubmit: () => void;
  onDismiss: () => void;
  onDone: () => void;
}

const STATUS_CONFIG = {
  review: {
    icon: ArrowRightLeft,
    iconClass: 'text-foreground',
    bgClass: 'bg-muted/10',
    heading: 'Review Swap',
    description: 'Please review your swap details before confirming.',
    announcement: 'Review your swap details.',
  },
  pending: {
    icon: Loader2,
    iconClass: 'text-amber-500 animate-spin',
    bgClass: 'bg-amber-500/10',
    heading: 'Waiting for wallet\u2026',
    description: 'Waiting for wallet signature. Please approve the transaction in your wallet.',
    announcement: 'Waiting for wallet signature.',
  },
  submitted: {
    icon: Loader2,
    iconClass: 'text-amber-500 animate-spin',
    bgClass: 'bg-amber-500/10',
    heading: 'Awaiting confirmation',
    description: 'Transaction submitted, awaiting confirmation on the network.',
    announcement: 'Transaction submitted, awaiting confirmation.',
  },
  confirmed: {
    icon: CheckCircle2,
    iconClass: 'text-green-500',
    bgClass: 'bg-green-500/10',
    heading: 'Swap confirmed',
    description: 'Your swap has been confirmed on the Stellar network.',
    announcement: 'Swap confirmed successfully.',
  },
  failed: {
    icon: XCircle,
    iconClass: 'text-destructive',
    bgClass: 'bg-destructive/10',
    heading: 'Swap failed',
    description: 'The swap could not be completed. You can try again or dismiss.',
    announcement: 'Swap failed.',
  },
  dropped: {
    icon: Clock,
    iconClass: 'text-muted-foreground',
    bgClass: 'bg-muted/20',
    heading: 'Transaction timed out',
    description: 'The transaction was not confirmed within the deadline. You can resubmit or dismiss.',
    announcement: 'Transaction timed out.',
  },
} as const;

const IN_FLIGHT_STATUSES: Array<TransactionStatus | 'review'> = ['pending', 'submitted'];

export function TransactionConfirmationModal({
  isOpen,
  status,
  txHash,
  errorMessage,
  tradeParams,
  onConfirm,
  onCancel,
  onTryAgain,
  onResubmit,
  onDismiss,
  onDone,
}: TransactionConfirmationModalProps) {
  const primaryActionRef = useRef<HTMLButtonElement>(null);
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;
  const isInFlight = IN_FLIGHT_STATUSES.includes(status);

  // Move focus to primary action button on each status transition
  useEffect(() => {
    if (isOpen && primaryActionRef.current) {
      primaryActionRef.current.focus();
    }
  }, [isOpen, status]);

  // Suppress Escape key during in-flight states
  const handleOpenChange = (open: boolean) => {
    if (!open && isInFlight) return; // block close during pending/submitted
    if (!open) {
      if (status === 'confirmed') onDone();
      else if (status === 'failed' || status === 'dropped') onDismiss();
      else if (status === 'review') onCancel();
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-[420px] p-0 overflow-hidden border-border/40 bg-background/95 backdrop-blur-xl rounded-[32px] shadow-2xl"
        aria-describedby="tcm-state-desc"
      >
        {/* Visually hidden state description for aria-describedby */}
        <p id="tcm-state-desc" className="sr-only">
          {config.description}
        </p>

        {/* aria-live region for screen reader announcements */}
        <div aria-live="polite" aria-atomic="true" className="sr-only">
          {config.announcement}
        </div>

        <div className="p-8 space-y-6">
          <DialogHeader>
            <div
              className={cn(
                'mx-auto w-16 h-16 rounded-full flex items-center justify-center mb-4',
                config.bgClass
              )}
            >
              <Icon className={cn('h-8 w-8', config.iconClass)} />
            </div>
            <DialogTitle className="text-2xl font-bold text-center tracking-tight">
              {config.heading}
            </DialogTitle>
            <DialogDescription className="text-center text-muted-foreground pt-2">
              {status === 'failed' && errorMessage
                ? errorMessage
                : config.description}
            </DialogDescription>
          </DialogHeader>

          {/* Trade summary (shown in review and confirmed states) */}
          {tradeParams && (status === 'review' || status === 'confirmed') && (
            <div className="bg-muted/30 rounded-2xl p-4 border border-border/20 space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">You pay</span>
                <span className="font-medium">
                  {tradeParams.fromAmount} {tradeParams.fromAsset}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">You receive</span>
                <span className="font-medium">
                  {tradeParams.toAmount} {tradeParams.toAsset}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Min received</span>
                <span className="font-medium">{tradeParams.minReceived}</span>
              </div>
            </div>
          )}

          {/* Confirmed: show tx hash */}
          {status === 'confirmed' && txHash && (
            <p className="text-center text-xs text-muted-foreground break-all">
              Tx: {txHash}
            </p>
          )}
        </div>

        <DialogFooter className="flex flex-col sm:flex-row gap-3 p-8 bg-muted/10 border-t border-border/20">
          {status === 'review' && (
            <>
              <Button
                ref={primaryActionRef}
                onClick={onConfirm}
                className="flex-1 h-12 rounded-xl font-bold shadow-lg"
              >
                Confirm Swap
              </Button>
              <Button
                variant="outline"
                onClick={onCancel}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                Cancel
              </Button>
            </>
          )}

          {status === 'pending' && (
            <Button
              ref={primaryActionRef}
              variant="outline"
              onClick={onCancel}
              className="flex-1 h-12 rounded-xl font-bold"
            >
              Cancel
            </Button>
          )}

          {status === 'submitted' && (
            <Button
              ref={primaryActionRef}
              variant="outline"
              disabled
              className="flex-1 h-12 rounded-xl font-bold opacity-50"
            >
              Processing&hellip;
            </Button>
          )}

          {status === 'confirmed' && (
            <Button
              ref={primaryActionRef}
              onClick={onDone}
              className="flex-1 h-12 rounded-xl font-bold shadow-lg shadow-green-500/20"
            >
              Done
            </Button>
          )}

          {status === 'failed' && (
            <>
              <Button
                ref={primaryActionRef}
                onClick={onTryAgain}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                Try Again
              </Button>
              <Button
                variant="outline"
                onClick={onDismiss}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                Dismiss
              </Button>
            </>
          )}

          {status === 'dropped' && (
            <>
              <Button
                ref={primaryActionRef}
                onClick={onResubmit}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                Resubmit
              </Button>
              <Button
                variant="outline"
                onClick={onDismiss}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                Dismiss
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
