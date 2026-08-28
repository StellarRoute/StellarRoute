import { describe, it, expect } from 'vitest';
import { API_ERROR_CODES, ApiErrorCode } from './types.js';

describe('ApiErrorCode drift check', () => {
  it('detects drift in error codes', () => {
    // 31 backend codes + 3 SDK-specific codes
    const EXPECTED_CODES = [
      'internal_error',
      'bad_request',
      'not_found',
      'validation_error',
      'rate_limit_exceeded',
      'overloaded',
      'unauthorized',
      'invalid_asset',
      'invalid_amount',
      'invalid_slippage',
      'invalid_asset_format',
      'no_route',
      'not_executable',
      'stale_market_data',
      'not_implemented',
      'quote_not_found',
      'quote_expired',
      'duplicate_quote',
      'dependency_unavailable',
      'unsupported_execution_mode',
      'unsupported_route',
      'cctp_not_enabled',
      'unsupported_corridor',
      'invalid_finality',
      'invalid_recipient',
      'fee_quote_unavailable',
      'attestation_pending',
      'attestation_expired',
      'mint_retryable',
      'transfer_not_found',
      'provider_killed',
      'network_error',
      'network_mismatch',
      'unknown_error',
    ];

    const currentCodes = [
      ...API_ERROR_CODES,
      'network_error',
      'network_mismatch',
      'unknown_error',
    ];

    // Every code in the union (simulated via API_ERROR_CODES + SDK codes) appears in the expected list
    for (const code of currentCodes) {
      expect(EXPECTED_CODES).toContain(code);
    }

    // Every code in the expected list appears in the union
    for (const code of EXPECTED_CODES) {
      expect(currentCodes).toContain(code);
    }

    // Exact length match to catch duplicates or omissions
    expect(currentCodes.length).toBe(EXPECTED_CODES.length);
  });
});
