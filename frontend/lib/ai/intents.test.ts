import { describe, expect, it } from 'vitest';
import {
  parseIntent,
  validateIntent,
  IntentValidationError,
  ConvertIntent,
  SubscribeIntent,
} from './intents';

describe('AI Agent Intent Types and Validator', () => {
  describe('convert intent', () => {
    it('successfully parses a valid convert intent', () => {
      const raw = {
        type: 'convert',
        fromAsset: 'XLM',
        toAsset: 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        amount: '100.5',
        slippageTolerance: '0.01',
      };

      const parsed = parseIntent(raw) as ConvertIntent;
      expect(parsed.type).toBe('convert');
      expect(parsed.fromAsset).toBe('XLM');
      expect(parsed.toAsset).toBe('USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN');
      expect(parsed.amount).toBe('100.5');
      expect(parsed.slippageTolerance).toBe('0.01');
    });

    it('rejects a convert intent with a missing amount', () => {
      const raw = {
        type: 'convert',
        fromAsset: 'XLM',
        toAsset: 'USDC',
      };

      expect(() => parseIntent(raw)).toThrow(IntentValidationError);
      expect(() => parseIntent(raw)).toThrow(/convert intent requires non-empty "amount"/);

      const result = validateIntent(raw);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toMatch(/convert intent requires non-empty "amount"/);
      }
    });

    it('rejects a convert intent with an empty or whitespace amount', () => {
      const raw = {
        type: 'convert',
        fromAsset: 'XLM',
        toAsset: 'USDC',
        amount: '   ',
      };

      expect(() => parseIntent(raw)).toThrow(IntentValidationError);
    });

    it('rejects a convert intent with missing fromAsset or toAsset', () => {
      expect(() =>
        parseIntent({
          type: 'convert',
          toAsset: 'USDC',
          amount: '10',
        }),
      ).toThrow(/convert intent requires non-empty "fromAsset"/);

      expect(() =>
        parseIntent({
          type: 'convert',
          fromAsset: 'XLM',
          amount: '10',
        }),
      ).toThrow(/convert intent requires non-empty "toAsset"/);
    });
  });

  describe('send intent', () => {
    it('successfully parses a valid send intent', () => {
      const raw = {
        type: 'send',
        recipient: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        asset: 'XLM',
        amount: '50',
        memo: 'Payment for services',
      };

      const parsed = parseIntent(raw);
      expect(parsed.type).toBe('send');
      if (parsed.type === 'send') {
        expect(parsed.recipient).toBe(raw.recipient);
        expect(parsed.asset).toBe('XLM');
        expect(parsed.amount).toBe('50');
        expect(parsed.memo).toBe('Payment for services');
      }
    });

    it('rejects send intent with missing required fields', () => {
      expect(() =>
        parseIntent({
          type: 'send',
          asset: 'XLM',
          amount: '50',
        }),
      ).toThrow(/send intent requires non-empty "recipient"/);
    });
  });

  describe('receive intent', () => {
    it('successfully parses a valid receive intent', () => {
      const raw = {
        type: 'receive',
        asset: 'USDC',
        amount: '25',
      };

      const parsed = parseIntent(raw);
      expect(parsed.type).toBe('receive');
      if (parsed.type === 'receive') {
        expect(parsed.asset).toBe('USDC');
        expect(parsed.amount).toBe('25');
      }
    });

    it('allows receive intent with optional amount omitted', () => {
      const raw = {
        type: 'receive',
        asset: 'XLM',
      };

      const parsed = parseIntent(raw);
      expect(parsed.type).toBe('receive');
      if (parsed.type === 'receive') {
        expect(parsed.asset).toBe('XLM');
        expect(parsed.amount).toBeUndefined();
      }
    });
  });

  describe('bridge intent', () => {
    it('successfully parses a valid bridge intent', () => {
      const raw = {
        type: 'bridge',
        sourceAsset: 'USDC',
        sourceChain: 'stellar',
        destinationAsset: 'USDC',
        destinationChain: 'sepolia',
        amount: '200',
        recipient: '0x1234567890abcdef1234567890abcdef12345678',
      };

      const parsed = parseIntent(raw);
      expect(parsed.type).toBe('bridge');
      if (parsed.type === 'bridge') {
        expect(parsed.sourceAsset).toBe('USDC');
        expect(parsed.sourceChain).toBe('stellar');
        expect(parsed.destinationChain).toBe('sepolia');
        expect(parsed.amount).toBe('200');
        expect(parsed.recipient).toBe(raw.recipient);
      }
    });

    it('rejects bridge intent with missing chain or asset', () => {
      expect(() =>
        parseIntent({
          type: 'bridge',
          sourceAsset: 'USDC',
          destinationAsset: 'USDC',
          destinationChain: 'sepolia',
          amount: '200',
        }),
      ).toThrow(/bridge intent requires non-empty "sourceChain"/);
    });
  });

  describe('offramp intent', () => {
    it('successfully parses a valid offramp intent', () => {
      const raw = {
        type: 'offramp',
        sourceAsset: 'USDC',
        fiatCurrency: 'NGN',
        amount: '150',
        destinationAccount: '0123456789',
      };

      const parsed = parseIntent(raw);
      expect(parsed.type).toBe('offramp');
      if (parsed.type === 'offramp') {
        expect(parsed.sourceAsset).toBe('USDC');
        expect(parsed.fiatCurrency).toBe('NGN');
        expect(parsed.amount).toBe('150');
        expect(parsed.destinationAccount).toBe('0123456789');
      }
    });

    it('rejects offramp intent with missing fiatCurrency or amount', () => {
      expect(() =>
        parseIntent({
          type: 'offramp',
          sourceAsset: 'USDC',
          amount: '150',
        }),
      ).toThrow(/offramp intent requires non-empty "fiatCurrency"/);
    });
  });

  describe('subscribe intent', () => {
    it('successfully parses a valid subscribe intent with payee, asset, amount, interval, and maxPayments', () => {
      const raw = {
        type: 'subscribe',
        payee: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        asset: 'USDC',
        amount: '10',
        interval: 'monthly',
        maxPayments: 12,
      };

      const parsed = parseIntent(raw) as SubscribeIntent;
      expect(parsed.type).toBe('subscribe');
      expect(parsed.payee).toBe(raw.payee);
      expect(parsed.asset).toBe('USDC');
      expect(parsed.amount).toBe('10');
      expect(parsed.interval).toBe('monthly');
      expect(parsed.maxPayments).toBe(12);
    });

    it('rejects subscribe intent if maxPayments is missing or not a positive number', () => {
      expect(() =>
        parseIntent({
          type: 'subscribe',
          payee: 'GA5Z...',
          asset: 'USDC',
          amount: '10',
          interval: 'monthly',
          maxPayments: 0,
        }),
      ).toThrow(/subscribe intent requires positive number for "maxPayments"/);

      expect(() =>
        parseIntent({
          type: 'subscribe',
          payee: 'GA5Z...',
          asset: 'USDC',
          amount: '10',
          interval: 'monthly',
        }),
      ).toThrow(/subscribe intent requires positive number for "maxPayments"/);
    });
  });

  describe('balance intent', () => {
    it('successfully parses a balance inquiry', () => {
      const parsed = parseIntent({
        type: 'balance',
        asset: 'XLM',
      });

      expect(parsed.type).toBe('balance');
      if (parsed.type === 'balance') {
        expect(parsed.asset).toBe('XLM');
      }
    });
  });

  describe('invalid input handling', () => {
    it('rejects null or non-object input', () => {
      expect(() => parseIntent(null)).toThrow(/Intent must be a non-null object/);
      expect(() => parseIntent('string')).toThrow(/Intent must be a non-null object/);
    });

    it('rejects unsupported intent type', () => {
      expect(() => parseIntent({ type: 'unknown_type' })).toThrow(
        /Unsupported intent type: "unknown_type"/,
      );
    });
  });
});
