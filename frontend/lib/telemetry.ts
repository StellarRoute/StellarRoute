export const ROUTE_SELECTED_EVENT_NAME = 'stellarroute:route-selected';

export interface RouteTelemetryEvent {
  venue: string;
  hopCount: number;
}

export interface TelemetryConfig {
  enabled: boolean;
}

export const telemetryConfig: TelemetryConfig = {
  enabled: process.env.NEXT_PUBLIC_TELEMETRY_ENABLED !== 'false',
};

export type TelemetryEventVersion = '1.0.0';
export type RouteEventName = 'route_view' | 'route_select' | 'route_confirm';

export interface RouteTelemetryPayload {
  fromAssetCode?: string;
  toAssetCode?: string;
  routeLength?: number;
  priceImpactTier?: 'low' | 'medium' | 'high' | 'severe';
  hasDex?: boolean;
  hasAmm?: boolean;
  venue?: string;
  hopCount?: number;
}

export interface TelemetryEvent {
  version: TelemetryEventVersion;
  eventName: RouteEventName;
  timestamp: number;
  payload: RouteTelemetryPayload;
}

export function getPriceImpactTier(impactPct: string | number): RouteTelemetryPayload['priceImpactTier'] {
  const num = typeof impactPct === 'string' ? parseFloat(impactPct) : impactPct;
  if (isNaN(num)) return 'low';
  if (num >= 5) return 'severe';
  if (num >= 2) return 'high';
  if (num >= 0.5) return 'medium';
  return 'low';
}

export function emitRouteEvent(venue: string, hopCount: number): void {
  if (process.env.NEXT_PUBLIC_TELEMETRY_ENABLED === 'false') {
    return;
  }

  if (typeof window === 'undefined' || typeof CustomEvent === 'undefined') {
    return;
  }

  window.dispatchEvent(
    new CustomEvent<RouteTelemetryEvent>(ROUTE_SELECTED_EVENT_NAME, {
      detail: { venue, hopCount },
    }),
  );
}
