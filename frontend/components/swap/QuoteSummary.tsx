"use client";

interface QuoteSummaryProps {
  rate: string;
  fee: string;
  priceImpact: string;
}

export function QuoteSummary({ rate, fee, priceImpact }: QuoteSummaryProps) {
  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Rate</span>
        <span className="font-medium truncate max-w-[60%]">{rate}</span>
      </div>
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Network Fee</span>
        <span className="font-medium truncate max-w-[60%]">{fee}</span>
      </div>
      <div className="flex justify-between items-center text-sm">
        <span className="text-muted-foreground">Price Impact</span>
        <span className="font-medium text-emerald-500 min-w-0 truncate max-w-[60%]">{priceImpact}</span>
      </div>
    </div>
  );
}
