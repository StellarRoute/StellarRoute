/**
 * Multi-step plan parser (AI-28).
 *
 * Splits a prompt on "then" into an ordered list of existing intents. It only
 * parses — no step runs here. If any step is unclear the whole plan becomes a
 * clarification so nothing is partially spent.
 */

import { parseIntent, type Clarification, type Intent } from './parse';

export interface Plan {
  kind: 'plan';
  steps: Intent[];
}

export type PlanResult = Plan | Clarification;

const STEP_SEPARATOR = /\s*,?\s+(?:and\s+)?then\s+/i;
const BARE_CASH_OUT = /^cash\s+out(?:\s+to\s+(\S+))?$/i;

function parseStep(text: string, previous: Intent | undefined): Intent | Clarification {
  // "… then cash out" carries the amount and asset over from a preceding bridge.
  const bare = text.match(BARE_CASH_OUT);
  if (bare) {
    if (previous?.type === 'bridge') {
      return {
        type: 'cash_out',
        amount: previous.amount,
        asset: previous.asset,
        ...(bare[1] ? { currency: bare[1] } : {}),
      };
    }
    return { kind: 'clarification', message: 'Specify how much to cash out.' };
  }
  return parseIntent(text);
}

/** Splits a prompt into its step phrases ("… then …"). */
export function splitSteps(text: string): string[] {
  return text
    .trim()
    .split(STEP_SEPARATOR)
    .map((part) => part.trim())
    .filter(Boolean);
}

export function parsePlan(text: string): PlanResult {
  const parts = splitSteps(text);

  if (parts.length === 0) {
    return { kind: 'clarification', message: "I'm not sure what you'd like to do." };
  }

  const steps: Intent[] = [];
  for (const [index, part] of parts.entries()) {
    const result = parseStep(part, steps[index - 1]);
    if ('kind' in result) {
      return {
        kind: 'clarification',
        message:
          parts.length > 1
            ? `Step ${index + 1} ("${part}") is unclear: ${result.message}`
            : result.message,
      };
    }
    steps.push(result);
  }

  return { kind: 'plan', steps };
}
