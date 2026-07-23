import { PathStep } from '@/types';
import { Card } from '@/components/ui/card';
import { getAssetCode, parseSource } from '@/lib/route-helpers';
import { SwapViewState } from '@/components/shared/ViewState';

export interface RouteRowProps {
  step?: PathStep;
  isLoading?: boolean;
  error?: string;
}

export function RouteRow({ step, isLoading, error }: RouteRowProps) {
  if (isLoading) {
    return (
      <SwapViewState
        variant="loading"
        title="Loading Route"
        description="Fetching best route step details…"
        className="p-3"
      />
    );
  }

  if (error) {
    return (
      <SwapViewState
        variant="error"
        title="Route Error"
        description={error}
        className="p-3 border-destructive"
      />
    );
  }

  if (!step) {
    return (
      <SwapViewState
        variant="empty"
        title="No Route Step"
        description="No route step available to display."
        className="p-3"
      />
    );
  }

  const from = getAssetCode(step.from_asset);
  const to = getAssetCode(step.to_asset);
  const sourceMeta = parseSource(step.source);

  return (
    <Card className="p-3">
      <div className="flex justify-between items-center gap-2">
        <div>
          <div className="text-sm font-semibold">{from} → {to}</div>
          <div className="text-xs text-muted-foreground">Price {step.price}</div>
        </div>
        <div className="text-xs rounded px-2 py-1 bg-muted/40">
          {sourceMeta.isSDEX ? 'SDEX' : sourceMeta.poolName || 'AMM'}
        </div>
      </div>
    </Card>
  );
}