export const ROUTE_SELECTED_EVENT_NAME = 'stellarroute:route-selected';

export interface RouteTelemetryEvent {
  venue: string;
  hopCount: number;
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
