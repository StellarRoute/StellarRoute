import { Skeleton } from "@/components/ui/skeleton";

import { useSwapI18n } from "@/lib/swap-i18n";
import { QuoteSummarySkeleton } from "./QuoteSummarySkeleton";

interface QuoteSummaryProps {
  rate: string;
  fee: string;
  priceImpact: string;
  isLoading?: boolean;
  error?: string;
}

export function QuoteSummary({
  rate,
  fee,
  priceImpact,
  isLoading = false,
  error,
}: QuoteSummaryProps) {
  const { t } = useSwapI18n();

  if (isLoading) {
    return <QuoteSummarySkeleton />;
  }

  if (error) {
    return (
      <div className="rounded-xl border border-destructive/50 p-4 bg-destructive/10 text-destructive text-sm font-medium">
        {error}
      </div>
    );
  }

  const displayRate = rate?.trim() || null;
  const displayFee = fee?.trim() || null;
  const displayPriceImpact = priceImpact?.trim() || null;

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      {rate && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">{t("swap.quote.rate")}</span>
          <span className="font-medium truncate max-w-[60%]">{rate}</span>
        </div>
      )}
      {fee && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">{t("swap.quote.networkFee")}</span>
          <span className="font-medium truncate max-w-[60%]">{fee}</span>
        </div>
      )}
      {priceImpact && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">{t("swap.quote.priceImpact")}</span>
          <span className="font-medium text-emerald-500 min-w-0 truncate max-w-[60%]">
            {priceImpact}
          </span>
        </div>
      )}
    </div>
  );
}
