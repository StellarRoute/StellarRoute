import { describe, expect, it } from 'vitest';
import {
  validatePreviewRecipientAddress,
  validateStellarRecipient,
} from './recipient-validation';

const VALID_G =
  'GAH4OLUSPDOHMFUENP2X3YUIIML7AE62ZOLHZE5X6C622WXPXLH2MNJT';
const VALID_MUXED =
  'MAH4OLUSPDOHMFUENP2X3YUIIML7AE62ZOLHZE5X6C622WXPXLH2MAAAAAAAAAAAABCGY';
const VALID_CONTRACT = 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4';

describe('validateStellarRecipient', () => {
  it('accepts valid G account via StrKey', () => {
    expect(validateStellarRecipient(VALID_G).valid).toBe(true);
  });

  it('accepts valid muxed M account via StrKey', () => {
    expect(validateStellarRecipient(VALID_MUXED).valid).toBe(true);
  });

  it('rejects contract C addresses as preview-only', () => {
    const result = validateStellarRecipient(VALID_CONTRACT);
    expect(result.valid).toBe(false);
    if (!result.valid) {
      expect(result.message).toMatch(/preview-only/i);
    }
  });

  it('rejects invalid Stellar address', () => {
    expect(validateStellarRecipient('not-an-address').valid).toBe(false);
  });
});

describe('validatePreviewRecipientAddress', () => {
  it('accepts EVM address', () => {
    expect(
      validatePreviewRecipientAddress(
        'evm',
        '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0'
      ).valid
    ).toBe(true);
  });

  it('accepts Solana address', () => {
    expect(
      validatePreviewRecipientAddress(
        'solana',
        'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK'
      ).valid
    ).toBe(true);
  });

  it('accepts Bitcoin testnet address', () => {
    expect(
      validatePreviewRecipientAddress(
        'bitcoin',
        'tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx'
      ).valid
    ).toBe(true);
  });

  it('accepts TRON address', () => {
    expect(
      validatePreviewRecipientAddress('tron', 'TLyqzVGLV1srkB7dToTAEqgDSfPtXRJZYH')
        .valid
    ).toBe(true);
  });
});
