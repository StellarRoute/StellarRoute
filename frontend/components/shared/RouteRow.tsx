import { PathStep } from '@/types';
import { Card } from '@/components/ui/card';
import { getAssetCode, parseSource } from '@/lib/route-helpers';

export interface RouteRowProps {
  step?: PathStep;
  isLoading?: boolean;
  error?: string;
}

export function RouteRow({ step, isLoading, error }: RouteRowProps) {
  if (isLoading) {
    return (
      <Card className="p-3" role="status" aria-busy="true">
        <div className="text-sm text-muted-foreground">Loading route row…</div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-3 border-destructive">
        <div className="text-sm text-destructive">Route row error: {error}</div>
      </Card>
    );
  }

  if (!step) {
    return (
      <Card className="p-3">
        <div className="text-sm text-muted-foreground">No route step</div>
      </Card>
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
        <div className="text-xs rounded px-2 py-1 bg-muted/40">{sourceMeta.isSDEX ? 'SDEX' : sourceMeta.poolName || 'AMM'}</div>
      </div>
    </Card>
  );
}
