import { StellarRouteApiError } from '@/lib/api/client';
import type { ParsedAgentIntent } from '@/app/ai/IntentPreviewCard';

export interface ValidateIntentRequest {
  type: string;
  amount: string;
  asset: string;
  recipient?: string;
  destination?: string;
  source?: string;
  currency?: string;
  recurrence?: string;
}

export interface ValidateIntentResponse {
  amount: string;
  type: string;
}

function isAiAgentEnabled(): boolean {
  if (typeof window !== 'undefined') {
    const flags = (window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> })
      .__STELLAR_ROUTE_FLAGS__;
    if (flags?.ai_agent !== undefined) {
      return Boolean(flags.ai_agent);
    }
  }
  return process.env.NEXT_PUBLIC_AI_AGENT === 'true' || process.env.NEXT_PUBLIC_AI_AGENT === '1';
}

export function resolveOfframpSourceId(intent: ParsedAgentIntent): string | null {
  if (intent.kind !== 'offramp') {
    return null;
  }

  const value = (intent.fromAsset ?? '').trim().toLowerCase();

  if (!value) {
    return 'stellar-usdc';
  }

  const aliases: Record<string, string> = {
    usdc: 'stellar-usdc',
    'stellar-usdc': 'stellar-usdc',
    xlm: 'stellar-xlm',
    'stellar-xlm': 'stellar-xlm',
    eth: 'eth-usdc',
    'eth-usdc': 'eth-usdc',
    base: 'base-usdc',
    'base-usdc': 'base-usdc',
    arbitrum: 'base-usdc',
    'arb-usdc': 'base-usdc',
    sol: 'sol-usdc',
    'sol-usdc': 'sol-usdc',
  };

  return aliases[value] ?? 'stellar-usdc';
}

export function buildOfframpRedirectUrl(intent: ParsedAgentIntent): string | null {
  if (intent.kind !== 'offramp') {
    return null;
  }

  const amount = (intent.amount ?? '').trim();
  const source = resolveOfframpSourceId(intent);

  if (!amount || !source) {
    return null;
  }

  return `/offramp?${new URLSearchParams({ amount, source }).toString()}`;
}

export async function validateIntent(
  intent: ParsedAgentIntent,
): Promise<ValidateIntentResponse | null> {
  if (!isAiAgentEnabled()) {
    return null;
  }

  try {
    const request: ValidateIntentRequest = {
      type: intent.kind,
      amount: intent.amount || '0',
      asset: intent.fromAsset || '',
      recipient: intent.recipient,
      destination: intent.destinationChain,
      source: intent.sourceChain,
      currency: intent.fiatCurrency,
      recurrence: intent.interval,
    };

    const response = await fetch('/api/v1/agent/intents/validate', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });

    if (response.status === 404) {
      // Agent feature is disabled
      return null;
    }

    if (!response.ok) {
      const errorData = (await response.json()) as { error?: { code?: string; message?: string } };
      throw new Error(errorData.error?.message || `Validation failed with status ${response.status}`);
    }

    const data = (await response.json()) as { data: ValidateIntentResponse };
    return data.data;
  } catch (error) {
    console.error('Failed to validate intent:', error);
    throw error;
  }
}
