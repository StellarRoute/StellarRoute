import { Skeleton } from "@/components/ui/skeleton";

import { QuoteSummarySkeleton } from "./QuoteSummarySkeleton";

interface QuoteSummaryProps {
  rate: string;
  fee: string;
  priceImpact: string;
  isLoading?: boolean;
}

export function QuoteSummary({
  rate,
  fee,
  priceImpact,
  isLoading = false,
}: QuoteSummaryProps) {
  if (isLoading) {
    return <QuoteSummarySkeleton />;
  }

  const displayRate = rate?.trim() || null;
  const displayFee = fee?.trim() || null;
  const displayPriceImpact = priceImpact?.trim() || null;

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      {rate && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Rate</span>
          <span className="font-medium truncate max-w-[60%]">{rate}</span>
        </div>
      )}
      {fee && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Network Fee</span>
          <span className="font-medium truncate max-w-[60%]">{fee}</span>
        </div>
      )}
      {priceImpact && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Price Impact</span>
          <span className="font-medium text-emerald-500 min-w-0 truncate max-w-[60%]">
            {priceImpact}
          </span>
        </div>
      )}
    </div>
  );
}