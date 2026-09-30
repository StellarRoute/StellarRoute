/**
 * Agent-specific error copy for trader-facing AI agent failures.
 *
 * Follows the same voice and tone as the trader-error-copy-style-guide.md:
 * - Direct and calm: describe what happened without blame
 * - Focus on next action: every message includes a recovery step
 * - No catastrophic language
 * - Headline, explanation, recovery action
 *
 * This file is **additive-only**. Existing swap error codes in
 * frontend/lib/api/trader-error-copy.ts must NOT be changed.
 */

export interface AgentErrorCopy {
  headline: string;
  explanation: string;
  recoveryAction: string;
  ctaLabel: string;
}

export type AgentErrorCode =
  | 'wallet_declined'
  | 'quote_unavailable'
  | 'bridge_off'
  | 'balance_too_low'
  | 'bad_address';

const DEFAULT_AGENT_COPY: AgentErrorCopy = {
  headline: 'We could not complete this action',
  explanation: 'Something unexpected happened while processing your request.',
  recoveryAction: 'Try again or rephrase what you would like to do.',
  ctaLabel: 'Try again',
};

const AGENT_ERROR_COPY: Record<AgentErrorCode, AgentErrorCopy> = {
  wallet_declined: {
    headline: 'Wallet action was declined',
    explanation:
      'Your wallet did not confirm the request needed to proceed with this action.',
    recoveryAction:
      'Open your wallet, approve the request, and try again.',
    ctaLabel: 'Open wallet and retry',
  },
  quote_unavailable: {
    headline: 'Quote not available for this route',
    explanation:
      'Current liquidity and pricing could not produce a quote for the requested route.',
    recoveryAction:
      'Try a different amount, asset pair, or destination chain.',
    ctaLabel: 'Adjust route details',
  },
  bridge_off: {
    headline: 'Bridge is not available right now',
    explanation:
      'The cross-chain bridge service is temporarily disabled.',
    recoveryAction:
      'Try again later or choose a different route that does not use this bridge.',
    ctaLabel: 'Check back later',
  },
  balance_too_low: {
    headline: 'Your balance is too low',
    explanation:
      'Your account does not have enough funds to cover this action plus network fees.',
    recoveryAction:
      'Add funds to your account or lower the amount, then try again.',
    ctaLabel: 'Add funds',
  },
  bad_address: {
    headline: 'Destination address is not valid',
    explanation:
      'The address format could not be recognized for this network.',
    recoveryAction:
      'Check the destination address and update it before trying again.',
    ctaLabel: 'Review address',
  },
};

/**
 * Return the trader-facing copy for a given agent error code.
 * Falls back to a safe default for unrecognised codes.
 */
export function getAgentErrorCopy(
  code: string | undefined | null,
): AgentErrorCopy {
  if (code && code in AGENT_ERROR_COPY) {
    return AGENT_ERROR_COPY[code as AgentErrorCode];
  }
  return DEFAULT_AGENT_COPY;
}

/**
 * Format agent error copy into a single display line.
 */
export function toAgentErrorLine(copy: AgentErrorCopy): string {
  return `${copy.headline}. ${copy.explanation} ${copy.recoveryAction}`;
}