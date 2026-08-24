import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { useViewState } from '@/components/shared/ViewState';
import type { PathStep } from '@/types';

export interface QuoteCardProps {
  fromAmount?: string;
  toAmount?: string;
  price?: string;
  slippage?: number;
  path?: PathStep[];
  isLoading?: boolean;
  error?: string;
}

export function QuoteCard({ fromAmount, toAmount, price, slippage, path, isLoading, error }: QuoteCardProps) {
  const view = useViewState(
    fromAmount && toAmount && price ? { fromAmount, toAmount, price } : null,
    isLoading ?? false,
    error,
    {
      loadingMessage: "Loading quote…",
      emptyMessage: "No quote data",
      emptyDescription: "Enter an amount to see a quote.",
    },
  );

  if (view.state !== "ready") return view.component;

  return (
    <Card className="p-4">
      <div className="mb-2 text-sm font-semibold">Quote Details</div>
      <div className="grid grid-cols-1 gap-2 text-sm">
        <div>From: {fromAmount}</div>
        <div>To: {toAmount}</div>
        <div>Price: {price}</div>
        {typeof slippage === 'number' && <div>Slippage tolerance: {slippage}%</div>}
        {path && path.length > 0 && (
          <div>
            Route: <Badge variant="secondary" className="text-xs">{path.length} hop{path.length === 1 ? '' : 's'}</Badge>
          </div>
        )}
      </div>
    </Card>
  );
}
