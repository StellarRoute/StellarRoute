export type AgentTelemetryEventName =
  | 'agent_intent_parsed'
  | 'agent_validation_failed'
  | 'agent_confirm'
  | 'agent_cancel';

export type AgentIntentKind =
  | 'convert'
  | 'send'
  | 'receive'
  | 'bridge'
  | 'offramp'
  | 'subscribe'
  | 'balance'
  | string;

export interface AgentTelemetryPayload {
  eventName: AgentTelemetryEventName;
  kind: AgentIntentKind;
}

export const AGENT_TELEMETRY_EVENT = 'stellarroute:agent';

function isTelemetryEnabled(): boolean {
  return process.env.NEXT_PUBLIC_TELEMETRY_ENABLED !== 'false';
}

/**
 * Emits an agent telemetry event with the event name and intent kind only.
 * In accordance with privacy specifications, memo text, destination addresses,
 * and amounts are strictly omitted.
 */
export function emitAgentTelemetry(
  eventName: AgentTelemetryEventName,
  kind: AgentIntentKind,
): AgentTelemetryPayload | undefined {
  if (!isTelemetryEnabled()) {
    return undefined;
  }

  // Payload keys must strictly be only the event name and kind
  const payload: AgentTelemetryPayload = {
    eventName,
    kind,
  };

  // Provide non-enumerable fallback getter for consumers expecting .event
  Object.defineProperty(payload, 'event', {
    get() {
      return eventName;
    },
    enumerable: false,
    configurable: true,
  });

  if (typeof window !== 'undefined' && typeof CustomEvent !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent<AgentTelemetryPayload>(AGENT_TELEMETRY_EVENT, {
        detail: payload,
      }),
    );
    window.dispatchEvent(
      new CustomEvent<AgentTelemetryPayload>(eventName, {
        detail: payload,
      }),
    );
  }

  return payload;
}
