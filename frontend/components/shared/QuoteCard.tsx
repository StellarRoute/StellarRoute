import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { PathStep } from '@/types';
import { SwapViewState } from './ViewState';

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
  if (isLoading) {
    return (
      <Card className="p-4">
        <SwapViewState kind="quote" variant="loading" />
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-4 border-destructive">
        <SwapViewState
          kind="quote"
          variant="error"
          description={error}
        />
      </Card>
    );
  }

  if (!fromAmount || !toAmount || !price) {
    return (
      <Card className="p-4">
        <SwapViewState kind="quote" variant="empty" />
      </Card>
    );
  }

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
