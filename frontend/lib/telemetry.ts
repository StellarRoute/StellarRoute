export const ROUTE_SELECTED_EVENT_NAME = 'stellarroute:route-selected';
export const SWAP_FUNNEL_EVENT_NAME = 'stellarroute:swap-funnel';

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

export type SwapFunnelEventName =
  | 'quote_requested'
  | 'confirm_clicked'
  | 'swap_submitted'
  | 'swap_finalized'
  | 'swap_failed';

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

/** PII-safe swap funnel payload — no wallet addresses or exact trade amounts. */
export interface SwapFunnelPayload {
  quoteId?: string;
  routeId?: string;
  fromAssetCode?: string;
  toAssetCode?: string;
  hopCount?: number;
  priceImpactTier?: RouteTelemetryPayload['priceImpactTier'];
  /** Coarse failure class only (e.g. build | sign | submit | config). */
  failureStage?: string;
}

export interface TelemetryEvent {
  version: TelemetryEventVersion;
  eventName: RouteEventName;
  timestamp: number;
  payload: RouteTelemetryPayload;
}

export interface SwapFunnelTelemetryEvent {
  version: TelemetryEventVersion;
  eventName: SwapFunnelEventName;
  timestamp: number;
  payload: SwapFunnelPayload;
}

export function getPriceImpactTier(impactPct: string | number): RouteTelemetryPayload['priceImpactTier'] {
  const num = typeof impactPct === 'string' ? parseFloat(impactPct) : impactPct;
  if (isNaN(num)) return 'low';
  if (num >= 5) return 'severe';
  if (num >= 2) return 'high';
  if (num >= 0.5) return 'medium';
  return 'low';
}

function isTelemetryEnabled(): boolean {
  return process.env.NEXT_PUBLIC_TELEMETRY_ENABLED !== 'false';
}

export function emitRouteEvent(venue: string, hopCount: number): void {
  if (!isTelemetryEnabled()) {
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

export function emitSwapFunnelEvent(
  eventName: SwapFunnelEventName,
  payload: SwapFunnelPayload = {},
): void {
  if (!isTelemetryEnabled()) {
    return;
  }

  if (typeof window === 'undefined' || typeof CustomEvent === 'undefined') {
    return;
  }

  const detail: SwapFunnelTelemetryEvent = {
    version: '1.0.0',
    eventName,
    timestamp: Date.now(),
    payload,
  };

  window.dispatchEvent(
    new CustomEvent<SwapFunnelTelemetryEvent>(SWAP_FUNNEL_EVENT_NAME, {
      detail,
    }),
  );
}
