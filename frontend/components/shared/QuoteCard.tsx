import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { PathStep } from '@/types';

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
      <Card className="p-4" role="status" aria-busy="true">
        <div className="text-sm text-muted-foreground">Loading quote…</div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-4 border-destructive">
        <div className="text-sm text-destructive">Quote error: {error}</div>
      </Card>
    );
  }

  if (!fromAmount || !toAmount || !price) {
    return (
      <Card className="p-4">
        <div className="text-sm text-muted-foreground">No quote data</div>
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
