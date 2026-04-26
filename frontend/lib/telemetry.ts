/**
 * Client-side feature telemetry for route selection behavior
 */

export interface TelemetryConfig {
  enabled: boolean;
}

export const telemetryConfig: TelemetryConfig = {
  // Can be disabled via environment variable
  enabled: process.env.NEXT_PUBLIC_TELEMETRY_ENABLED !== 'false',
};

// Event Schema Version 1.0.0
export type TelemetryEventVersion = '1.0.0';

export type RouteEventName = 'route_view' | 'route_select' | 'route_confirm';

export interface RouteTelemetryPayload {
  // Only non-sensitive data is allowed
  fromAssetCode: string;
  toAssetCode: string;
  routeLength: number;
  priceImpactTier: 'low' | 'medium' | 'high' | 'severe';
  hasDex: boolean;
  hasAmm: boolean;
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

export function emitRouteEvent(
  eventName: RouteEventName,
  payload: RouteTelemetryPayload
) {
  if (!telemetryConfig.enabled) return;

  const event: TelemetryEvent = {
    version: '1.0.0',
    eventName,
    timestamp: Date.now(),
    payload,
  };

  // In a real implementation this would send to an analytics endpoint.
  // For the scope of this feature, we emit a DOM event that could be picked up
  // by an analytics integration, and log to debug.
  console.debug('[Telemetry]', event);
  
  if (typeof window !== 'undefined') {
    try {
      window.dispatchEvent(new CustomEvent('stellar_route_telemetry', { detail: event }));
    } catch (e) {
      // Ignore
    }
  }
}
