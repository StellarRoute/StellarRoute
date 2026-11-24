import { describe, expect, it } from 'vitest';
import {
  getAgentErrorCopy,
  toAgentErrorLine,
  type AgentErrorCode,
} from './errors';

describe('getAgentErrorCopy', () => {
  const cases: { code: AgentErrorCode; headline: string; ctaLabel: string }[] = [
    {
      code: 'wallet_declined',
      headline: 'Wallet action was declined',
      ctaLabel: 'Open wallet and retry',
    },
    {
      code: 'quote_unavailable',
      headline: 'Quote not available for this route',
      ctaLabel: 'Adjust route details',
    },
    {
      code: 'bridge_off',
      headline: 'Bridge is not available right now',
      ctaLabel: 'Check back later',
    },
    {
      code: 'balance_too_low',
      headline: 'Your balance is too low',
      ctaLabel: 'Add funds',
    },
    {
      code: 'bad_address',
      headline: 'Destination address is not valid',
      ctaLabel: 'Review address',
    },
  ];

  for (const { code, headline, ctaLabel } of cases) {
    it(`maps ${code} to trader-facing copy with headline "${headline}"`, () => {
      const copy = getAgentErrorCopy(code);
      expect(copy.headline).toBe(headline);
      expect(copy.ctaLabel).toBe(ctaLabel);
      expect(copy.explanation.length).toBeGreaterThan(0);
      expect(copy.recoveryAction.length).toBeGreaterThan(0);
    });
  }

  it('falls back to safe default for null', () => {
    const copy = getAgentErrorCopy(null);
    expect(copy.headline).toBe('We could not complete this action');
  });

  it('falls back to safe default for undefined', () => {
    const copy = getAgentErrorCopy(undefined);
    expect(copy.headline).toBe('We could not complete this action');
  });

  it('falls back to safe default for unknown code', () => {
    const copy = getAgentErrorCopy('unknown_code_here');
    expect(copy.headline).toBe('We could not complete this action');
  });

  it('does not use blame or panic language in any mapped copy', () => {
    const blameOrPanicWords = [
      'you failed',
      'invalid user',
      'fatal',
      'catastrophic',
      'critical failure',
    ];
    const codes: AgentErrorCode[] = [
      'wallet_declined',
      'quote_unavailable',
      'bridge_off',
      'balance_too_low',
      'bad_address',
    ];

    for (const code of codes) {
      const copy = getAgentErrorCopy(code);
      const combined = `${copy.headline} ${copy.explanation} ${copy.recoveryAction}`.toLowerCase();

      for (const word of blameOrPanicWords) {
        expect(combined).not.toContain(word);
      }
    }
  });

  it('each case has headline, explanation, and recovery action', () => {
    const codes: AgentErrorCode[] = [
      'wallet_declined',
      'quote_unavailable',
      'bridge_off',
      'balance_too_low',
      'bad_address',
    ];

    for (const code of codes) {
      const copy = getAgentErrorCopy(code);
      expect(copy.headline).toBeTruthy();
      expect(copy.explanation).toBeTruthy();
      expect(copy.recoveryAction).toBeTruthy();
      expect(copy.ctaLabel).toBeTruthy();
    }
  });
});

describe('toAgentErrorLine', () => {
  it('formats copy into a single display line', () => {
    const copy = getAgentErrorCopy('wallet_declined');
    const line = toAgentErrorLine(copy);
    expect(line).toContain('Wallet action was declined.');
    expect(line).toContain(copy.explanation);
    expect(line).toContain(copy.recoveryAction);
  });
});