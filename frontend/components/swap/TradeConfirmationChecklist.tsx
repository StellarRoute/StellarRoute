'use client';

import {
  CheckCircle2,
  AlertCircle,
  Clock,
  Wallet,
  TrendingUp,
  Shield,
} from 'lucide-react';
import { cn } from '@/lib/utils';

export interface ChecklistItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  status: 'pending' | 'pass' | 'fail' | 'warning';
  message?: string;
}

export interface TradeConfirmationChecklistProps {
  items: ChecklistItem[];
  isReady: boolean;
  onConfirm: () => void;
  confirmDisabled?: boolean;
}

const STATUS_CONFIG = {
  pending: {
    bgClass: 'bg-muted/20 border-border/40',
    iconClass: 'text-muted-foreground',
    textClass: 'text-muted-foreground',
  },
  pass: {
    bgClass: 'bg-green-500/10 border-green-500/30',
    iconClass: 'text-green-600 dark:text-green-400',
    textClass: 'text-green-700 dark:text-green-300',
  },
  fail: {
    bgClass: 'bg-destructive/10 border-destructive/30',
    iconClass: 'text-destructive',
    textClass: 'text-destructive',
  },
  warning: {
    bgClass: 'bg-amber-500/10 border-amber-500/30',
    iconClass: 'text-amber-600 dark:text-amber-400',
    textClass: 'text-amber-700 dark:text-amber-300',
  },
};

const STATUS_PRIORITY: Record<ChecklistItem['status'], number> = {
  fail: 3,
  warning: 2,
  pending: 1,
  pass: 0,
};

export function TradeConfirmationChecklist({
  items,
  isReady,
  onConfirm,
  confirmDisabled = false,
}: TradeConfirmationChecklistProps) {
  // Sort by priority: fails first, then warnings, pending, passes
  const sortedItems = [...items].sort(
    (a, b) =>
      (STATUS_PRIORITY[b.status] ?? 0) - (STATUS_PRIORITY[a.status] ?? 0)
  );

  // Determine overall status
  const hasFail = items.some((item) => item.status === 'fail');
  const hasWarning = items.some((item) => item.status === 'warning');
  const allPass = items.every((item) => item.status === 'pass');

  const overallStatus = hasFail ? 'fail' : hasWarning ? 'warning' : allPass ? 'pass' : 'pending';

  return (
    <div className="space-y-4">
      {/* Header */}
      <div>
        <h3 className="text-sm font-semibold mb-1">Pre-Submission Checklist</h3>
        <p className="text-xs text-muted-foreground">
          All checks must pass before you can proceed.
        </p>
      </div>

      {/* Checklist Items */}
      <div className="space-y-2">
        {sortedItems.map((item) => {
          const config = STATUS_CONFIG[item.status];
          const Icon = item.icon;
          const StatusIcon =
            item.status === 'pass'
              ? CheckCircle2
              : item.status === 'fail'
                ? AlertCircle
                : item.status === 'warning'
                  ? AlertCircle
                  : Clock;

          return (
            <div
              key={item.id}
              className={cn(
                'flex items-start gap-3 p-3 rounded-lg border transition-colors',
                config.bgClass
              )}
            >
              <div className="flex-shrink-0 pt-0.5">
                <Icon className={cn('h-4 w-4', config.iconClass)} />
              </div>

              <div className="flex-1 min-w-0">
                <p className={cn('text-sm font-medium', config.textClass)}>
                  {item.label}
                </p>
                {item.message && (
                  <p className="text-xs mt-0.5 opacity-80">{item.message}</p>
                )}
              </div>

              <div className="flex-shrink-0 pt-0.5">
                <StatusIcon className={cn('h-4 w-4', config.iconClass)} />
              </div>
            </div>
          );
        })}
      </div>

      {/* Overall Status Summary */}
      <div
        className={cn(
          'p-3 rounded-lg border text-sm',
          overallStatus === 'pass'
            ? 'bg-green-500/10 border-green-500/30 text-green-700 dark:text-green-300'
            : overallStatus === 'fail'
              ? 'bg-destructive/10 border-destructive/30 text-destructive'
              : overallStatus === 'warning'
                ? 'bg-amber-500/10 border-amber-500/30 text-amber-700 dark:text-amber-300'
                : 'bg-muted/20 border-border/40 text-muted-foreground'
        )}
      >
        {overallStatus === 'pass' ? (
          <span className="font-medium">✓ All checks passed. Ready to swap.</span>
        ) : overallStatus === 'fail' ? (
          <span className="font-medium">
            ✗ Some checks failed. Please resolve before proceeding.
          </span>
        ) : overallStatus === 'warning' ? (
          <span className="font-medium">
            ⚠ Warnings detected. Proceed at your own risk.
          </span>
        ) : (
          <span className="font-medium">Running checks…</span>
        )}
      </div>

      {/* Confirm Button */}
      <button
        onClick={onConfirm}
        disabled={!isReady || confirmDisabled || hasFail}
        className={cn(
          'w-full py-2.5 px-4 rounded-lg font-medium text-sm transition-all duration-200',
          !isReady || confirmDisabled || hasFail
            ? 'bg-muted text-muted-foreground cursor-not-allowed opacity-50'
            : 'bg-primary text-primary-foreground hover:bg-primary/90 active:scale-[0.98]'
        )}
      >
        {!isReady ? 'Running Checks…' : 'Confirm & Proceed'}
      </button>
    </div>
  );
}

/**
 * Hook to generate checklist items based on swap state
 */
export function useTradeChecklist({
  fromAmount,
  fromBalance,
  slippage,
  quoteAge,
  routeFreshness,
  walletConnected,
}: {
  fromAmount?: string;
  fromBalance?: string;
  slippage?: number;
  quoteAge?: number; // in milliseconds
  routeFreshness?: 'fresh' | 'stale' | 'missing';
  walletConnected?: boolean;
}): { items: ChecklistItem[]; isReady: boolean } {
  const items: ChecklistItem[] = [];

  // 1. Wallet Connected
  if (typeof walletConnected === 'boolean') {
    items.push({
      id: 'wallet',
      label: 'Wallet Connected',
      icon: Wallet,
      status: walletConnected ? 'pass' : 'fail',
      message: walletConnected
        ? 'Wallet is ready'
        : 'Connect a wallet to proceed',
    });
  }

  // 2. Balance Check
  if (fromAmount && fromBalance) {
    const amount = parseFloat(fromAmount);
    const balance = parseFloat(fromBalance);
    const hasSufficientBalance = !isNaN(amount) && !isNaN(balance) && amount <= balance;

    items.push({
      id: 'balance',
      label: 'Sufficient Balance',
      icon: Shield,
      status: hasSufficientBalance ? 'pass' : 'fail',
      message: hasSufficientBalance
        ? `${fromAmount} available`
        : `Insufficient balance (need ${fromAmount}, have ${fromBalance})`,
    });
  }

  // 3. Slippage Tolerance
  if (typeof slippage === 'number') {
    const isValidSlippage = slippage >= 0.01 && slippage <= 100;
    const isWarning = slippage > 5 || slippage < 0.1;

    items.push({
      id: 'slippage',
      label: 'Slippage Tolerance',
      icon: TrendingUp,
      status: !isValidSlippage ? 'fail' : isWarning ? 'warning' : 'pass',
      message: !isValidSlippage
        ? `Invalid slippage: ${slippage}%`
        : isWarning
          ? `Slippage is ${slippage > 5 ? 'very high' : 'very low'}: ${slippage}%`
          : `Slippage: ${slippage}%`,
    });
  }

  // 4. Quote Freshness
  if (routeFreshness) {
    items.push({
      id: 'route',
      label: 'Route Freshness',
      icon: Clock,
      status:
        routeFreshness === 'missing'
          ? 'fail'
          : routeFreshness === 'stale'
            ? 'warning'
            : 'pass',
      message:
        routeFreshness === 'missing'
          ? 'No route available'
          : routeFreshness === 'stale'
            ? `Quote is stale (${quoteAge ? Math.round(quoteAge / 1000) : '?'}s old)`
            : 'Route is fresh',
    });
  }

  const isReady =
    items.length > 0 &&
    !items.some((item) => item.status === 'fail' || item.status === 'pending');

  return { items, isReady };
}
