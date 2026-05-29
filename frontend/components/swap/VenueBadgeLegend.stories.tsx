import { VenueBadgeLegend } from './VenueBadgeLegend';
import { RouteDisplay } from './RouteDisplay';

const meta = {
  title: 'Swap / VenueBadgeLegend',
};
export default meta;


/**
 * Showcases the VenueBadgeLegend component in isolation inside a themed card.
 */
export const Isolation = () => {
  return (
    <div className="max-w-sm mx-auto p-6 border border-border/80 bg-popover text-popover-foreground rounded-xl shadow-lg">
      <VenueBadgeLegend />
    </div>
  );
};

/**
 * Demonstrates the VenueBadgeLegend integrated into the Best Route RouteDisplay header.
 * Click the 'Info' icon next to 'Best Route' in the rendered card to open the popover.
 */
export const IntegratedInRouteDisplay = () => {
  return (
    <div className="max-w-md mx-auto p-6 space-y-4">
      <RouteDisplay amountOut="125.5000" confidenceScore={92} volatility="low" />
    </div>
  );
};
