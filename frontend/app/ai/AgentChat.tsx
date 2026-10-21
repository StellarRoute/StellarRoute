'use client';

import * as React from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { IntentPreviewCard, type ParsedAgentIntent } from './IntentPreviewCard';
import { emitAgentTelemetry, type AgentIntentKind } from './telemetry';
import { validateIntent } from '@/lib/ai/client';
import { loadCap } from '@/lib/ai/cap';
import { parsePlan, splitSteps } from '@/lib/ai/plan';
import type { Intent } from '@/lib/ai/parse';
import { PlanRunner } from './PlanRunner';
import { SpendingCapSettings } from './SpendingCapSettings';
import { ArrowRight } from 'lucide-react';

export function parsePromptToIntent(text: string): ParsedAgentIntent | null {
  const trimmed = text.trim();
  const lower = trimmed.toLowerCase();

  if (!trimmed) return null;

  if (lower.startsWith('swap') || lower.startsWith('convert')) {
    const match = trimmed.match(/(?:swap|convert)\s+(\d+(?:\.\d+)?)\s+(\S+)\s+to\s+(\S+)/i);
    if (match) {
      return {
        kind: 'convert',
        amount: match[1],
        fromAsset: match[2],
        toAsset: match[3],
        summary: `Convert ${match[1]} ${match[2]} to ${match[3]}`,
      };
    }
    return {
      kind: 'convert',
      summary: 'Convert assets',
    };
  }

  if (lower.startsWith('send')) {
    const match = trimmed.match(/send\s+(\d+(?:\.\d+)?)\s+(\S+)\s+to\s+(\S+)/i);
    if (match) {
      return {
        kind: 'send',
        amount: match[1],
        fromAsset: match[2],
        recipient: match[3],
        summary: `Send ${match[1]} ${match[2]} to ${match[3]}`,
      };
    }
    return {
      kind: 'send',
      summary: 'Send payment',
    };
  }

  if (lower.startsWith('receive')) {
    return {
      kind: 'receive',
      summary: 'Receive payment into wallet',
    };
  }

  if (lower.startsWith('bridge')) {
    const match = trimmed.match(/bridge\s+(\d+(?:\.\d+)?)\s+(\S+)\s+to\s+(\S+)/i);
    if (match) {
      return {
        kind: 'bridge',
        amount: match[1],
        fromAsset: match[2],
        destinationChain: match[3],
        summary: `Bridge ${match[1]} ${match[2]} to ${match[3]}`,
      };
    }
    return {
      kind: 'bridge',
      summary: 'Cross-chain bridge transfer',
    };
  }

  if (lower.startsWith('cash out') || lower.startsWith('offramp')) {
    const match = trimmed.match(/(?:cash\s+out|offramp)\s+(\d+(?:\.\d+)?)\s+(\S+)\s+to\s+(\S+)/i);
    if (match) {
      return {
        kind: 'offramp',
        amount: match[1],
        fromAsset: match[2],
        fiatCurrency: match[3],
        summary: `Cash out ${match[1]} ${match[2]} to ${match[3]}`,
      };
    }
    return {
      kind: 'offramp',
      summary: 'Offramp crypto to fiat',
    };
  }

  if (lower.startsWith('pay') || lower.startsWith('subscribe')) {
    const match = trimmed.match(/(?:pay|subscribe)\s+(\d+(?:\.\d+)?)\s+(\S+)\s+(monthly|weekly|daily)\s+to\s+(\S+)/i);
    if (match) {
      return {
        kind: 'subscribe',
        amount: match[1],
        fromAsset: match[2],
        interval: match[3],
        recipient: match[4],
        summary: `Subscribe ${match[1]} ${match[2]} (${match[3]}) to ${match[4]}`,
      };
    }
    return {
      kind: 'subscribe',
      summary: 'Recurring subscription schedule',
    };
  }

  if (lower.startsWith('balance')) {
    return {
      kind: 'balance',
      summary: 'Check account balance',
    };
  }

  return {
    kind: 'balance',
    summary: trimmed,
  };
}

export function AgentChat() {
  const [input, setInput] = React.useState('');
  const [activeIntent, setActiveIntent] = React.useState<ParsedAgentIntent | null>(null);
  const [validationError, setValidationError] = React.useState<string | null>(null);
  const [isValidating, setIsValidating] = React.useState(false);
  const [planSteps, setPlanSteps] = React.useState<Intent[] | null>(null);
  const [usdcCap, setUsdcCap] = React.useState<number | null>(null);

  React.useEffect(() => {
    setUsdcCap(loadCap());
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    // Multi-step prompts ("… then …") render as a plan; single steps keep the existing card.
    if (splitSteps(input).length > 1) {
      const plan = parsePlan(input);
      setActiveIntent(null);
      setPlanSteps(plan.kind === 'plan' ? plan.steps : null);
      setValidationError(plan.kind === 'plan' ? null : plan.message);
      setInput('');
      return;
    }
    setPlanSteps(null);

    const parsed = parsePromptToIntent(input);
    if (parsed) {
      emitAgentTelemetry('agent_intent_parsed', parsed.kind);

      // Try to validate the intent with the server
      setIsValidating(true);
      setValidationError(null);
      try {
        await validateIntent(parsed);
        // Validation succeeded or agent is disabled
        setActiveIntent(parsed);
      } catch (error) {
        // Validation failed, show error
        const errorMessage = error instanceof Error ? error.message : 'Validation failed';
        setValidationError(errorMessage);
        emitAgentTelemetry('agent_validation_failed', parsed.kind);
      } finally {
        setIsValidating(false);
      }
    }
    setInput('');
  };

  const handleCancel = () => {
    setActiveIntent(null);
    setValidationError(null);
  };

  return (
    <div className="space-y-6" data-testid="agent-chat-container">
      {validationError && (
        <div
          data-testid="validation-error"
          className="rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive"
        >
          {validationError}
        </div>
      )}

      {activeIntent && (
        <IntentPreviewCard
          intent={activeIntent}
          onCancel={handleCancel}
          usdcCap={usdcCap}
        />
      )}

      {planSteps && <PlanRunner steps={planSteps} usdcCap={usdcCap} />}

      <form onSubmit={handleSubmit} className="flex gap-2">
        <Input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Ask the agent (e.g. swap 10 XLM to USDC, send 5 USDC to G...)"
          className="flex-1"
          data-testid="agent-chat-input"
          disabled={isValidating}
        />
        <Button
          type="submit"
          data-testid="agent-chat-submit"
          className="gap-1.5"
          disabled={isValidating}
        >
          <ArrowRight className="h-4 w-4" />
          {isValidating ? 'Validating...' : 'Send'}
        </Button>
      </form>

      <SpendingCapSettings cap={usdcCap} onChange={setUsdcCap} />
    </div>
  );
}
