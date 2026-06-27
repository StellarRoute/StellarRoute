'use client';

import { useEffect, useRef } from 'react';
import { useReducedMotion } from '@/hooks/useReducedMotion';
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
import { PostSwapSuccessScreen } from './PostSwapSuccessScreen';
import { useSwapI18n } from '@/lib/swap-i18n';

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
  onSwapAgain?: () => void;
}

const STATUS_ICON_CONFIG = {
  review: {
    icon: ArrowRightLeft,
    iconClass: 'text-foreground',
    iconMotionClass: '',
    bgClass: 'bg-muted/10',
  },
  pending: {
    icon: Loader2,
    iconClass: 'text-amber-500',
    iconMotionClass: 'animate-spin',
    bgClass: 'bg-amber-500/10',
  },
  submitted: {
    icon: Loader2,
    iconClass: 'text-amber-500',
    iconMotionClass: 'animate-spin',
    bgClass: 'bg-amber-500/10',
  },
  confirmed: {
    icon: CheckCircle2,
    iconClass: 'text-green-500',
    iconMotionClass: '',
    bgClass: 'bg-green-500/10',
  },
  failed: {
    icon: XCircle,
    iconClass: 'text-destructive',
    iconMotionClass: '',
    bgClass: 'bg-destructive/10',
  },
  dropped: {
    icon: Clock,
    iconClass: 'text-muted-foreground',
    iconMotionClass: '',
    bgClass: 'bg-muted/20',
  },
} as const;

const IN_FLIGHT_STATUSES: Array<TransactionStatus | 'review'> = [
  'pending',
  'submitted',
];

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
  onSwapAgain,
}: TransactionConfirmationModalProps) {
  const primaryActionRef = useRef<HTMLButtonElement>(null);
  const prefersReducedMotion = useReducedMotion();
  const { t } = useSwapI18n();
  const iconConfig = STATUS_ICON_CONFIG[status];
  const Icon = iconConfig.icon;
  const isInFlight = IN_FLIGHT_STATUSES.includes(status);

  const statusTextConfig = {
    review: {
      heading: t('swap.confirm.review.heading'),
      description: t('swap.confirm.review.description'),
      announcement: t('swap.confirm.review.announcement'),
    },
    pending: {
      heading: t('swap.confirm.pending.heading'),
      description: t('swap.confirm.pending.description'),
      announcement: t('swap.confirm.pending.announcement'),
    },
    submitted: {
      heading: t('swap.confirm.submitted.heading'),
      description: t('swap.confirm.submitted.description'),
      announcement: t('swap.confirm.submitted.announcement'),
    },
    confirmed: {
      heading: t('swap.confirm.confirmed.heading'),
      description: t('swap.confirm.confirmed.description'),
      announcement: t('swap.confirm.confirmed.announcement'),
    },
    failed: {
      heading: t('swap.confirm.failed.heading'),
      description: t('swap.confirm.failed.description'),
      announcement: t('swap.confirm.failed.announcement'),
    },
    dropped: {
      heading: t('swap.confirm.dropped.heading'),
      description: t('swap.confirm.dropped.description'),
      announcement: t('swap.confirm.dropped.announcement'),
    },
  };
  const config = { ...iconConfig, ...statusTextConfig[status] };

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
              <Icon
                data-testid="tcm-spinner"
                className={cn(
                  'h-8 w-8',
                  config.iconClass,
                  !prefersReducedMotion && config.iconMotionClass
                )}
              />
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
                <span className="text-muted-foreground">{t('swap.confirm.summary.youPay')}</span>
                <span className="font-medium">
                  {tradeParams.fromAmount} {tradeParams.fromAsset}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">{t('swap.confirm.summary.youReceive')}</span>
                <span className="font-medium">
                  {tradeParams.toAmount} {tradeParams.toAsset}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">{t('swap.confirm.summary.minReceived')}</span>
                <span className="font-medium">{tradeParams.minReceived}</span>
              </div>
            </div>
          )}

          {/* Confirmed: dedicated post-swap success content */}
          {status === 'confirmed' && txHash && (
            <PostSwapSuccessScreen
              txHash={txHash}
              tradeParams={tradeParams}
              onDone={onDone}
              onSwapAgain={onSwapAgain}
            />
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
                {t('swap.confirm.cta.confirmSwap')}
              </Button>
              <Button
                variant="outline"
                onClick={onCancel}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                {t('swap.confirm.cta.cancel')}
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
              {t('swap.confirm.cta.cancel')}
            </Button>
          )}

          {status === 'submitted' && (
            <Button
              ref={primaryActionRef}
              variant="outline"
              disabled
              className="flex-1 h-12 rounded-xl font-bold opacity-50"
            >
              {t('swap.confirm.cta.processing')}
            </Button>
          )}

          {status === 'confirmed' && (
            <Button
              ref={primaryActionRef}
              onClick={onDone}
              className="flex-1 h-12 rounded-xl font-bold shadow-lg shadow-green-500/20"
            >
              {t('swap.confirm.cta.done')}
            </Button>
          )}

          {status === 'failed' && (
            <>
              <Button
                ref={primaryActionRef}
                onClick={onTryAgain}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                {t('swap.confirm.cta.tryAgain')}
              </Button>
              <Button
                variant="outline"
                onClick={onDismiss}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                {t('swap.confirm.cta.dismiss')}
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
                {t('swap.confirm.cta.resubmit')}
              </Button>
              <Button
                variant="outline"
                onClick={onDismiss}
                className="flex-1 h-12 rounded-xl font-bold"
              >
                {t('swap.confirm.cta.dismiss')}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
