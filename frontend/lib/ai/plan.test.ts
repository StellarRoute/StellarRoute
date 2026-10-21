import { describe, it, expect } from 'vitest';
import { parsePlan } from './plan';

const G = 'GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H';

describe('parsePlan', () => {
  it('bridge then cash out yields two ordered steps', () => {
    expect(parsePlan('bridge 20 USDC to sepolia then cash out')).toEqual({
      kind: 'plan',
      steps: [
        { type: 'bridge', amount: '20', asset: 'USDC', destination: 'sepolia' },
        { type: 'cash_out', amount: '20', asset: 'USDC' },
      ],
    });
  });

  it('swap then send yields two ordered steps', () => {
    expect(parsePlan(`swap 10 XLM to USDC then send 5 USDC to ${G}`)).toEqual({
      kind: 'plan',
      steps: [
        { type: 'swap', amount: '10', asset: 'XLM', destination: 'USDC' },
        { type: 'send', amount: '5', asset: 'USDC', recipient: G },
      ],
    });
  });

  it('accepts "and then" and ", then" separators', () => {
    const a = parsePlan(`swap 10 XLM to USDC and then send 5 USDC to ${G}`);
    const b = parsePlan(`swap 10 XLM to USDC, then send 5 USDC to ${G}`);
    expect(a).toEqual(b);
    expect(a.kind === 'plan' && a.steps).toHaveLength(2);
  });

  it('a single-step sentence yields one step', () => {
    expect(parsePlan('swap 10 XLM to USDC')).toEqual({
      kind: 'plan',
      steps: [{ type: 'swap', amount: '10', asset: 'XLM', destination: 'USDC' }],
    });
  });

  it('an unknown second step yields a clarification and no steps', () => {
    const result = parsePlan('swap 10 XLM to USDC then dance');
    expect(result.kind).toBe('clarification');
    expect(result).not.toHaveProperty('steps');
    expect(result.kind === 'clarification' && result.message).toMatch(/Step 2/);
  });

  it('a bare cash out without a preceding bridge asks for an amount', () => {
    const result = parsePlan('swap 10 XLM to USDC then cash out');
    expect(result).toEqual({
      kind: 'clarification',
      message: 'Step 2 ("cash out") is unclear: Specify how much to cash out.',
    });
  });

  it('a single unclear sentence keeps the underlying clarification', () => {
    expect(parsePlan('send 5 USDC')).toEqual({
      kind: 'clarification',
      message: 'Specify who to send to.',
    });
  });
});
