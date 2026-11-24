"use client";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ChevronDown, ArrowDown } from "lucide-react";
import { useOptionalSettings } from '@/components/providers/settings-provider';
import { formatAmount, getUserLocale } from '@/lib/formatting';
import { useRecentPairs, type RecentPair } from '@/hooks/useRecentPairs';

interface PairSelectorProps {
  payAmount: string;
  onPayAmountChange: (amount: string) => void;
  receiveAmount: string;
  payBalance?: number;
  receiveBalance?: number;
  /** Called when the user selects a recent pair from the history list. */
  onRecentPairSelect?: (pair: RecentPair) => void;
}

export function PairSelector({ 
  payAmount, 
  onPayAmountChange, 
  receiveAmount,
  payBalance = 1000,
  receiveBalance = 0,
  onRecentPairSelect,
}: PairSelectorProps) {
  const settings = useOptionalSettings();
  const locale = settings?.settings.locale ?? getUserLocale();
  const { recentPairs } = useRecentPairs();

  const formattedPayBalance = formatAmount(payBalance, locale, 2);
  const formattedReceiveBalance = formatAmount(receiveBalance, locale, 2);

  return (
    <div className="space-y-1 relative overflow-x-hidden">
      {recentPairs.length > 0 && (
        <nav aria-label="Recent pairs" className="mb-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1">
            Recent
          </p>
          <ul
            className="flex flex-wrap gap-1.5"
            data-testid="recent-pairs-list"
          >
            {recentPairs.map((pair) => (
              <li key={`${pair.baseAsset}|${pair.quoteAsset}`}>
                <button
                  type="button"
                  onClick={() => onRecentPairSelect?.(pair)}
                  className="inline-flex h-7 items-center rounded-full border border-border/60 bg-muted/50 px-3 text-xs font-medium text-muted-foreground hover:border-primary/50 hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 transition-colors"
                  aria-label={`Select recent pair ${pair.base}/${pair.quote}`}
                >
                  {pair.base}/{pair.quote}
                </button>
              </li>
            ))}
          </ul>
        </nav>
      )}

      <div className="bg-muted/50 rounded-xl p-4 border border-border/50 transition-colors focus-within:border-primary/50">
        <div className="text-sm font-medium text-muted-foreground mb-2">You Pay</div>
        <div className="flex items-center justify-between gap-4 max-[359px]:flex-col">
          <Input 
            type="number" 
            placeholder="0.00" 
            className="text-3xl font-medium p-0 border-0 shadow-none focus-visible:ring-ring/50 focus-visible:ring-[3px] bg-transparent h-auto max-w-[180px]"
            value={payAmount}
            onChange={(e) => onPayAmountChange(e.target.value)}
            inputMode="decimal"
            autoComplete="off"
            autoCorrect="off"
            aria-label="Pay amount"
          />
          <Button variant="secondary" className="rounded-full shadow-sm pr-2 pl-3 h-9" aria-label="Select token to pay">
            <span className="flex items-center gap-2">
              <div className="w-5 h-5 rounded-full bg-primary/20 flex items-center justify-center text-xs">X</div>
              <span className="font-semibold text-sm">XLM</span>
              <ChevronDown className="h-4 w-4 opacity-50" />
            </span>
          </Button>
        </div>
        <div className="text-sm text-muted-foreground mt-2">Balance: {formattedPayBalance}</div>
      </div>

      <div className="absolute left-1/2 -translate-x-1/2 top-1/2 -translate-y-1/2 z-10">
        <Button variant="outline" size="icon" className="h-8 w-8 rounded-full shadow-sm bg-background border-border" aria-label="Swap pay and receive tokens">
          <ArrowDown className="h-4 w-4" />
        </Button>
      </div>

      <div className="bg-muted/50 rounded-xl p-4 border border-border/50">
        <div className="text-sm font-medium text-muted-foreground mb-2">You Receive</div>
        <div className="flex items-center justify-between gap-4 max-[359px]:flex-col">
          <Input 
            type="text" 
            placeholder="0.00" 
            className="text-3xl font-medium p-0 border-0 shadow-none focus-visible:ring-ring/50 focus-visible:ring-[3px] bg-transparent h-auto max-w-[180px]"
            value={receiveAmount}
            readOnly
            aria-readonly="true"
            aria-label="Receive amount"
          />
          <Button variant="secondary" className="rounded-full shadow-sm pr-2 pl-3 h-9" aria-label="Select token to receive">
            <span className="flex items-center gap-2">
              <div className="w-5 h-5 rounded-full bg-primary/15 flex items-center justify-center text-xs text-primary">U</div>
              <span className="font-semibold text-sm">USDC</span>
              <ChevronDown className="h-4 w-4 opacity-50" />
            </span>
          </Button>
        </div>
        <div className="text-sm text-muted-foreground mt-2">Balance: {formattedReceiveBalance}</div>
      </div>
    </div>
  );
}
