import { Skeleton } from "@/components/ui/skeleton";

interface QuoteSummaryProps {
  rate: string;
  fee: string;
  priceImpact: string;
  isLoading?: boolean;
}

export function QuoteSummary({ rate, fee, priceImpact, isLoading = false }: QuoteSummaryProps) {
  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Rate</span>
        {isLoading ? (
          <Skeleton className="h-4 w-32" />
        ) : (
          <span className="font-medium truncate max-w-[60%]">{rate}</span>
        )}
      </div>
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Network Fee</span>
        {isLoading ? (
          <Skeleton className="h-4 w-16" />
        ) : (
          <span className="font-medium truncate max-w-[60%]">{fee}</span>
        )}
      </div>
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Price Impact</span>
        {isLoading ? (
          <Skeleton className="h-4 w-12" />
        ) : (
          <span className="font-medium text-emerald-500 min-w-0 truncate max-w-[60%]">
            {priceImpact}
          </span>
        )}
      </div>
    </div>
  );
}
