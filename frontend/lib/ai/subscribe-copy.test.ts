import { describe, it, expect } from 'vitest';
import {
  formatSubscriptionSchedule,
  renderSubscriptionPreview,
  parseSubscriptionPrompt,
  truncatePayee,
  CONFIRMATION_SAFETY_NOTICE,
  SubscriptionScheduleOptions,
} from './subscribe-copy';

describe('subscribe-copy', () => {
  const sampleStellarAddress =
    'GBZXN7PIRZGNMHGA728RGRFZAPPWN9G5FVK6KKWBHFHCGJ5BVK5RTR64';

  describe('formatSubscriptionSchedule', () => {
    it('should format a monthly subscription schedule with maxCount correctly', () => {
      const options: SubscriptionScheduleOptions = {
        amount: '15',
        asset: 'USDC',
        cadence: 'monthly',
        payee: sampleStellarAddress,
        maxCount: 12,
      };

      const copy = formatSubscriptionSchedule(options);

      expect(copy.cadenceLabel).toBe('Monthly');
      expect(copy.amountFormatted).toBe('15 USDC');
      expect(copy.payee).toBe(sampleStellarAddress);
      expect(copy.payeeShort).toBe('GBZX…TR64');
      expect(copy.maxCountLabel).toBe('Up to 12 payments');
      expect(copy.totalMaxFormatted).toBe('180 USDC maximum');
      expect(copy.headline).toBe(
        'Pay 15 USDC monthly to GBZX…TR64 (max 12 payments)'
      );
      expect(copy.confirmationNotice).toBe(CONFIRMATION_SAFETY_NOTICE);
      expect(copy.fullDescription).toContain(
        'Send 15 USDC monthly to GBZXN7PIRZGNMHGA728RGRFZAPPWN9G5FVK6KKWBHFHCGJ5BVK5RTR64 for a maximum of 12 payments'
      );
      expect(copy.fullDescription).toContain(CONFIRMATION_SAFETY_NOTICE);
    });

    it('should format a weekly subscription schedule correctly', () => {
      const options: SubscriptionScheduleOptions = {
        amount: 25.5,
        asset: 'XLM',
        cadence: 'weekly',
        payee: 'GABC1234567890XYZ',
        maxCount: 4,
      };

      const copy = formatSubscriptionSchedule(options);

      expect(copy.cadenceLabel).toBe('Weekly');
      expect(copy.amountFormatted).toBe('25.5 XLM');
      expect(copy.maxCountLabel).toBe('Up to 4 payments');
      expect(copy.totalMaxFormatted).toBe('102 XLM maximum');
      expect(copy.headline).toBe(
        'Pay 25.5 XLM weekly to GABC…0XYZ (max 4 payments)'
      );
      expect(copy.confirmationNotice).toBe(CONFIRMATION_SAFETY_NOTICE);
    });

    it('should handle single payment maxCount with singular grammar', () => {
      const options: SubscriptionScheduleOptions = {
        amount: '10',
        asset: 'USDC',
        cadence: 'monthly',
        payee: 'G1234567890',
        maxCount: 1,
      };

      const copy = formatSubscriptionSchedule(options);

      expect(copy.maxCountLabel).toBe('Up to 1 payment');
      expect(copy.headline).toContain('max 1 payment');
      expect(copy.fullDescription).toContain('for a maximum of 1 payment');
    });

    it('should format recurring subscriptions with no maxCount specified', () => {
      const options: SubscriptionScheduleOptions = {
        amount: '50',
        asset: 'USDC',
        cadence: 'monthly',
        payee: 'GSTELLARRECEIVER',
      };

      const copy = formatSubscriptionSchedule(options);

      expect(copy.maxCountLabel).toBe('Recurring until canceled');
      expect(copy.totalMaxFormatted).toBeNull();
      expect(copy.headline).toBe('Pay 50 USDC monthly to GSTE…IVER');
      expect(copy.fullDescription).toContain(
        'Send 50 USDC monthly to GSTELLARRECEIVER'
      );
      expect(copy.fullDescription).toContain(CONFIRMATION_SAFETY_NOTICE);
    });

    it('should include optional memo in the full description', () => {
      const options: SubscriptionScheduleOptions = {
        amount: '15',
        asset: 'USDC',
        cadence: 'monthly',
        payee: 'G1234567890',
        memo: 'Dev team stipend',
      };

      const copy = formatSubscriptionSchedule(options);
      expect(copy.fullDescription).toContain('with memo "Dev team stipend"');
    });
  });

  describe('renderSubscriptionPreview', () => {
    it('should render a multiline formatted preview block containing all required elements', () => {
      const options: SubscriptionScheduleOptions = {
        amount: '15',
        asset: 'USDC',
        cadence: 'monthly',
        payee: sampleStellarAddress,
        maxCount: 12,
      };

      const preview = renderSubscriptionPreview(options);

      expect(preview).toContain(
        'Schedule: Pay 15 USDC monthly to GBZX…TR64 (max 12 payments)'
      );
      expect(preview).toContain('Cadence: Monthly');
      expect(preview).toContain('Amount per payment: 15 USDC');
      expect(preview).toContain(`Payee: ${sampleStellarAddress}`);
      expect(preview).toContain('Duration: Up to 12 payments');
      expect(preview).toContain('Total ceiling: 180 USDC maximum');
      expect(preview).toContain(`Confirmation: ${CONFIRMATION_SAFETY_NOTICE}`);
    });
  });

  describe('truncatePayee', () => {
    it('should truncate a standard 56-character Stellar G-address', () => {
      expect(truncatePayee(sampleStellarAddress)).toBe('GBZX…TR64');
    });

    it('should allow custom front and back character counts', () => {
      expect(truncatePayee(sampleStellarAddress, 6, 6)).toBe('GBZXN7…5RTR64');
    });

    it('should return short strings unchanged', () => {
      expect(truncatePayee('GABC')).toBe('GABC');
      expect(truncatePayee('')).toBe('');
    });
  });

  describe('parseSubscriptionPrompt', () => {
    it('should parse "pay 15 USDC monthly to G... up to 12 payments"', () => {
      const prompt = `pay 15 USDC monthly to ${sampleStellarAddress} up to 12 payments`;
      const parsed = parseSubscriptionPrompt(prompt);

      expect(parsed).not.toBeNull();
      expect(parsed?.amount).toBe('15');
      expect(parsed?.asset).toBe('USDC');
      expect(parsed?.cadence).toBe('monthly');
      expect(parsed?.payee).toBe(sampleStellarAddress);
      expect(parsed?.maxCount).toBe(12);
    });

    it('should parse weekly cadence with max count', () => {
      const prompt = 'pay 25 XLM weekly to GABC123456789 for 4 weeks';
      const parsed = parseSubscriptionPrompt(prompt);

      expect(parsed).not.toBeNull();
      expect(parsed?.amount).toBe('25');
      expect(parsed?.asset).toBe('XLM');
      expect(parsed?.cadence).toBe('weekly');
      expect(parsed?.payee).toBe('GABC123456789');
      expect(parsed?.maxCount).toBe(4);
    });

    it('should parse prompt without max count', () => {
      const prompt = 'pay 50 USDC monthly to GDESTINATIONACCOUNT';
      const parsed = parseSubscriptionPrompt(prompt);

      expect(parsed).not.toBeNull();
      expect(parsed?.amount).toBe('50');
      expect(parsed?.asset).toBe('USDC');
      expect(parsed?.cadence).toBe('monthly');
      expect(parsed?.payee).toBe('GDESTINATIONACCOUNT');
      expect(parsed?.maxCount).toBeUndefined();
    });

    it('should return null for unrelated prompts', () => {
      expect(parseSubscriptionPrompt('swap 10 XLM to USDC')).toBeNull();
      expect(parseSubscriptionPrompt('send 5 USDC to G...')).toBeNull();
      expect(parseSubscriptionPrompt('')).toBeNull();
    });
  });

  describe('safety and constraints validation', () => {
    it('should state clearly that funds are not pulled on a timer', () => {
      expect(CONFIRMATION_SAFETY_NOTICE).toContain(
        'not be pulled automatically on a timer'
      );
      expect(CONFIRMATION_SAFETY_NOTICE).toContain(
        'requires manual confirmation'
      );
    });

    it('should handle zero and non-numeric amounts gracefully', () => {
      const copy = formatSubscriptionSchedule({
        amount: 'invalid',
        asset: 'USDC',
        cadence: 'monthly',
        payee: 'G123',
      });
      expect(copy.amountFormatted).toBe('0 USDC');
    });
  });
});
