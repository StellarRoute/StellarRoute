import { StrKey } from '@stellar/stellar-base';
import type { ChainFamily } from '@/lib/wallet/adapters';
import type { RecipientValidationResult } from './types';

const EVM_ADDRESS = /^0x[a-fA-F0-9]{40}$/;
const SOLANA_ADDRESS = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
const BITCOIN_ADDRESS =
  /^(?:bc1|tb1)[a-z0-9]{25,87}$|^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$/;
const TRON_ADDRESS = /^T[1-9A-HJ-NP-Za-km-z]{33}$/;

export function validateStellarRecipient(value: string): RecipientValidationResult {
  const trimmed = value.trim();
  if (!trimmed) {
    return { valid: false, message: 'Recipient address is required.' };
  }

  if (trimmed.startsWith('C')) {
    return {
      valid: false,
      message:
        'Contract (C…) recipients are preview-only — destination forwarding to Soroban contracts is not supported yet.',
    };
  }

  if (StrKey.isValidEd25519PublicKey(trimmed)) {
    return { valid: true };
  }

  if (StrKey.isValidMed25519PublicKey(trimmed)) {
    return { valid: true };
  }

  return {
    valid: false,
    message:
      'Enter a valid Stellar account (G…) or muxed account (M…) address.',
  };
}

/**
 * Shape-only validation for external chains — preview corridors only.
 * Does not imply executable routing or signing readiness.
 */
export function validatePreviewRecipientAddress(
  chainFamily: ChainFamily,
  value: string
): RecipientValidationResult {
  const trimmed = value.trim();
  if (!trimmed) {
    return { valid: false, message: 'Recipient address is required.' };
  }

  switch (chainFamily) {
    case 'stellar':
      return validateStellarRecipient(trimmed);
    case 'evm':
      return EVM_ADDRESS.test(trimmed)
        ? { valid: true }
        : {
            valid: false,
            message: 'Enter a valid EVM address (0x + 40 hex digits).',
          };
    case 'solana':
      return SOLANA_ADDRESS.test(trimmed)
        ? { valid: true }
        : {
            valid: false,
            message: 'Enter a valid Solana address (base58, 32–44 chars).',
          };
    case 'bitcoin':
      return BITCOIN_ADDRESS.test(trimmed)
        ? { valid: true }
        : {
            valid: false,
            message:
              'Enter a valid Bitcoin address (bc1…, tb1…, 1…, or 3… format).',
          };
    case 'tron':
      return TRON_ADDRESS.test(trimmed)
        ? { valid: true }
        : {
            valid: false,
            message: 'Enter a valid TRON address (T…, 34 characters).',
          };
    default:
      return { valid: false, message: 'Unsupported chain for recipient.' };
  }
}

/** @deprecated Use validatePreviewRecipientAddress — kept for import stability in tests. */
export function validateRecipientAddress(
  chainFamily: ChainFamily,
  value: string
): RecipientValidationResult {
  return validatePreviewRecipientAddress(chainFamily, value);
}
