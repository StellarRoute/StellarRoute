import { describe, expect, it, vi } from 'vitest';
import {
  createClarifier,
  intentForValidation,
  isReadyForValidation,
  missingClarificationFields,
  resolveTurn,
  type ClarifyResult,
} from './clarify';
import type { Intent } from './parse';

const RECIPIENT = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCD';

function questionOf(result: ClarifyResult): string {
  if (result.kind !== 'clarification') {
    throw new Error('expected a clarification turn');
  }
  return result.message;
}

function intentOf(result: ClarifyResult): Intent {
  if (result.kind !== 'intent') {
    throw new Error(`expected an intent turn, got: ${questionOf(result)}`);
  }
  return result.intent;
}

function pendingOf(result: ClarifyResult) {
  if (result.kind !== 'clarification') {
    throw new Error('expected a clarification turn');
  }
  return result.pending;
}

describe('resolveTurn', () => {
  it('asks for the amount when a swap sentence has no amount', () => {
    const result = resolveTurn('swap XLM to USDC');

    expect(result.kind).toBe('clarification');
    expect(questionOf(result)).toMatch(/how much/i);
    expect(result.kind === 'clarification' && result.field).toBe('amount');
    expect(pendingOf(result)).not.toBeNull();
    expect(pendingOf(result)?.values).toEqual({
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('fills the missing amount from the next message', () => {
    const first = resolveTurn('swap XLM to USDC');
    const second = resolveTurn('10', pendingOf(first));

    expect(second.kind).toBe('intent');
    expect(intentOf(second)).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
    expect(second.pending).toBeNull();
  });

  it('treats convert as a swap verb when asking for the amount', () => {
    const first = resolveTurn('convert XLM to USDC');

    expect(first.kind === 'clarification' && first.field).toBe('amount');
    expect(pendingOf(first)?.values).toEqual({
      asset: 'XLM',
      destination: 'USDC',
    });

    const second = resolveTurn('10', pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('accepts an answer with trailing punctuation', () => {
    const first = resolveTurn('swap XLM to USDC');
    const second = resolveTurn('10.', pendingOf(first));

    expect(intentOf(second)).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('asks again when the parser cannot express the answer', () => {
    const clarifier = createClarifier();
    const first = clarifier.submit('swap XLM to USDC');

    const decimal = clarifier.submit('10.5');

    expect(decimal.kind === 'clarification' && decimal.field).toBe('amount');
    expect(questionOf(decimal)).toBe(questionOf(first));
    expect(clarifier.pending?.asked).toBe('amount');
    expect(clarifier.pending?.values).toEqual({
      asset: 'XLM',
      destination: 'USDC',
    });
    expect(clarifier.submit('10').kind).toBe('intent');
  });

  it('asks for one of two missing fields at a time', () => {
    const first = resolveTurn('swap XLM');

    expect(pendingOf(first)?.missing).toEqual(['amount', 'destination']);
    expect(pendingOf(first)?.asked).toBe('amount');
    expect(missingClarificationFields(pendingOf(first))).toEqual([
      'amount',
      'destination',
    ]);

    const second = resolveTurn('10', pendingOf(first));
    expect(second.kind === 'clarification' && second.field).toBe('destination');
    expect(pendingOf(second)?.missing).toEqual(['destination']);
    expect(questionOf(second)).toContain('10 XLM');

    const third = resolveTurn('USDC', pendingOf(second));
    expect(intentOf(third)).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('exposes no intent until every field is filled', () => {
    const first = resolveTurn('swap XLM to USDC');
    const second = resolveTurn('10', pendingOf(first));

    expect(isReadyForValidation(first)).toBe(false);
    expect(intentForValidation(first)).toBeNull();
    expect('intent' in first).toBe(false);
    expect('intent' in second).toBe(true);
    expect(isReadyForValidation(second)).toBe(true);
  });

  it('clears the pending question on an unrelated sentence', () => {
    const clarifier = createClarifier();
    clarifier.submit('swap XLM to USDC');
    expect(clarifier.pending).not.toBeNull();

    const unrelated = clarifier.submit('do something weird');

    expect(unrelated.kind).toBe('clarification');
    expect(clarifier.pending).toBeNull();
    expect(pendingOf(unrelated)).toBeNull();

    const after = clarifier.submit('10');
    expect(after.kind).toBe('clarification');
    expect(intentForValidation(after)).toBeNull();
  });

  it('reads a multi-token reply as a new sentence', () => {
    const clarifier = createClarifier();
    clarifier.submit('swap XLM to USDC');

    const reply = clarifier.submit('10 USDC');

    expect(reply.kind).toBe('clarification');
    expect(clarifier.pending).toBeNull();
    expect(intentForValidation(reply)).toBeNull();
  });

  it('re-asks the open question for an empty message', () => {
    const clarifier = createClarifier();
    const first = clarifier.submit('swap XLM to USDC');

    const empty = clarifier.submit('   ');

    expect(questionOf(empty)).toBe(questionOf(first));
    expect(clarifier.pending).not.toBeNull();
    expect(intentOf(clarifier.submit('10'))).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('passes a complete sentence straight through', () => {
    const result = resolveTurn('swap 10 XLM to USDC');

    expect(intentOf(result)).toEqual({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
    expect(result.pending).toBeNull();
  });

  it('still requires a later confirm for auto-confirm wording', () => {
    const result = resolveTurn('swap 10 XLM to USDC and auto-confirm');

    expect(intentOf(result).type).toBe('swap');
    expect('confirmed' in intentOf(result)).toBe(false);
  });

  it('asks for a send recipient and completes on a Stellar address', () => {
    const first = resolveTurn('send 5 USDC');

    expect(first.kind === 'clarification' && first.field).toBe('recipient');
    expect(questionOf(first)).toContain('5 USDC');

    const second = resolveTurn(RECIPIENT, pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'send',
      amount: '5',
      asset: 'USDC',
      recipient: RECIPIENT,
    });
  });

  it('refuses a non-address answer for a send recipient', () => {
    const clarifier = createClarifier();
    clarifier.submit('send 5 USDC');

    const reply = clarifier.submit('my other wallet');

    expect(reply.kind).toBe('clarification');
    expect(clarifier.pending).toBeNull();
  });

  it('asks for a bridge destination chain', () => {
    const first = resolveTurn('bridge 25 USDC to');

    expect(first.kind === 'clarification' && first.field).toBe('destination');
    expect(questionOf(first)).toMatch(/chain/i);

    const second = resolveTurn('sepolia', pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'bridge',
      amount: '25',
      asset: 'USDC',
      destination: 'sepolia',
    });
  });

  it('asks for a bridge source chain when the sentence says from', () => {
    const first = resolveTurn('bridge 25 USDC from');

    expect(first.kind === 'clarification' && first.field).toBe('source');
    expect(pendingOf(first)?.order).toEqual(['amount', 'asset', 'source']);

    const second = resolveTurn('sepolia', pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'bridge',
      amount: '25',
      asset: 'USDC',
      source: 'sepolia',
    });
  });

  it('asks for a cash-out currency', () => {
    const first = resolveTurn('cash out 20 USDC to');

    expect(first.kind === 'clarification' && first.field).toBe('currency');

    const second = resolveTurn('naira', pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'cash_out',
      amount: '20',
      asset: 'USDC',
      currency: 'naira',
    });
  });

  it('asks for a pay recipient and keeps the recurrence', () => {
    const first = resolveTurn('pay 15 USDC monthly to');

    expect(first.kind === 'clarification' && first.field).toBe('recipient');
    expect(pendingOf(first)?.values.recurrence).toBe('monthly');

    const second = resolveTurn(RECIPIENT, pendingOf(first));
    expect(intentOf(second)).toEqual({
      type: 'pay',
      amount: '15',
      asset: 'USDC',
      recurrence: 'monthly',
      recipient: RECIPIENT,
    });
  });

  it('asks for a pay schedule when only the asset is known', () => {
    const first = resolveTurn('pay 15 USDC');

    expect(first.kind === 'clarification' && first.field).toBe('recurrence');

    const second = resolveTurn('monthly', pendingOf(first));
    expect(second.kind === 'clarification' && second.field).toBe('recipient');
  });

  it('refuses a chat word as a value', () => {
    const clarifier = createClarifier();
    clarifier.submit('swap XLM to USDC');

    const reply = clarifier.submit('thanks');

    expect(reply.kind).toBe('clarification');
    expect(clarifier.pending).toBeNull();
  });

  it('treats receive as complete with no question', () => {
    const result = resolveTurn('receive');

    expect(intentOf(result)).toEqual({
      type: 'receive',
      amount: '',
      asset: '',
    });
    expect(result.pending).toBeNull();
  });

  it('returns an open question with no fields for unknown text', () => {
    const result = resolveTurn('do something weird');

    expect(questionOf(result)).toBe("I'm not sure what you'd like to do.");
    expect(pendingOf(result)).toBeNull();
    expect(result.kind === 'clarification' && result.field).toBeNull();
  });
});

describe('createClarifier', () => {
  it('validates only the turn that completes the parse', () => {
    const validate = vi.fn();
    const clarifier = createClarifier();

    for (const message of ['swap XLM to USDC', '10']) {
      const intent = intentForValidation(clarifier.submit(message));
      if (intent) {
        validate(intent);
      }
    }

    expect(validate).toHaveBeenCalledTimes(1);
    expect(validate).toHaveBeenCalledWith({
      type: 'swap',
      amount: '10',
      asset: 'XLM',
      destination: 'USDC',
    });
  });

  it('clears the pending question on reset', () => {
    const clarifier = createClarifier();
    clarifier.submit('swap XLM to USDC');
    clarifier.reset();

    expect(clarifier.pending).toBeNull();
    expect(intentForValidation(clarifier.submit('10'))).toBeNull();
  });

  it('reports no missing fields for a fresh clarifier', () => {
    expect(missingClarificationFields(createClarifier().pending)).toEqual([]);
  });

  it('never calls the network', () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);

    try {
      const clarifier = createClarifier();
      for (const message of [
        'swap XLM to USDC',
        '10',
        'send 5 USDC',
        'what is my balance',
      ]) {
        clarifier.submit(message);
      }
    } finally {
      vi.unstubAllGlobals();
    }

    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
