import { describe, expect, it } from 'vitest';
import {
  isCctpPrimaryActionDisabled,
  resolveCctpCtaHint,
  resolveDestinationWalletSetupHint,
} from './cctpCtaHint';
import {
  cctpNeedsUserAction,
  resolveCctpNextActionNotice,
  resolveCctpNextActionToast,
} from './cctpNextActionNotice';

describe('cctpCtaHint', () => {
  it('asks to connect destination wallet without recipient override copy', () => {
    expect(
      resolveDestinationWalletSetupHint('stellar_to_evm', ''),
    ).toMatch(/Connect your ETH Sepolia wallet/i);
    expect(
      resolveCctpCtaHint({
        direction: 'stellar_to_evm',
        sourceAmount: '5',
        destRecipientAddress: '',
        bridgeReady: true,
        readinessLoading: false,
        sagaPrimaryDisabled: false,
      }),
    ).toMatch(/Connect your ETH Sepolia wallet/i);
  });

  it('disables CTA when destination wallet or amount is missing', () => {
    expect(
      isCctpPrimaryActionDisabled({
        direction: 'evm_to_stellar',
        sourceAmount: '5',
        destRecipientAddress: '',
        bridgeReady: true,
        readinessLoading: false,
        sagaPrimaryDisabled: false,
      }),
    ).toBe(true);
    expect(
      isCctpPrimaryActionDisabled({
        direction: 'evm_to_stellar',
        sourceAmount: '',
        destRecipientAddress: 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567',
        bridgeReady: true,
        readinessLoading: false,
        sagaPrimaryDisabled: false,
      }),
    ).toBe(true);
  });
});

describe('cctpNextActionNotice', () => {
  it('flags actionable CTA steps and resolves copy', () => {
    expect(cctpNeedsUserAction('mint', false)).toBe(true);
    expect(cctpNeedsUserAction('mint', true)).toBe(false);
    expect(cctpNeedsUserAction('none', false)).toBe(false);
    expect(resolveCctpNextActionNotice('burn')).toMatch(/Your turn/i);
    expect(resolveCctpNextActionToast('mint')).toMatch(/confirm receive/i);
    expect(resolveCctpNextActionToast('quote')).toBeNull();
  });
});
