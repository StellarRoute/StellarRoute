import { describe, it, expect } from 'vitest';
import {
  isCardHealth,
  isCardApplicationValidation,
  isCardAuthorization,
  isCardAuthorizationsResponse,
} from './types.js';

describe('isCardHealth', () => {
  it('accepts a valid CardHealth object', () => {
    expect(isCardHealth({ enabled: true })).toBe(true);
    expect(isCardHealth({ enabled: false })).toBe(true);
  });

  it('rejects non-objects and missing/invalid enabled field', () => {
    expect(isCardHealth(null)).toBe(false);
    expect(isCardHealth(undefined)).toBe(false);
    expect(isCardHealth('enabled')).toBe(false);
    expect(isCardHealth({})).toBe(false);
    expect(isCardHealth({ enabled: 'true' })).toBe(false);
  });
});

describe('isCardApplicationValidation', () => {
  it('accepts a valid validation result', () => {
    expect(
      isCardApplicationValidation({ valid: true, applicant_ref: 'ref-1' }),
    ).toBe(true);
    expect(
      isCardApplicationValidation({ valid: false, applicant_ref: '' }),
    ).toBe(true);
  });

  it('rejects objects missing or mistyping required fields', () => {
    expect(isCardApplicationValidation(null)).toBe(false);
    expect(isCardApplicationValidation({ valid: true })).toBe(false);
    expect(
      isCardApplicationValidation({ valid: 'yes', applicant_ref: 'ref-1' }),
    ).toBe(false);
    expect(
      isCardApplicationValidation({ valid: true, applicant_ref: 42 }),
    ).toBe(false);
  });
});

describe('isCardAuthorization', () => {
  it('accepts an authorization with only required fields', () => {
    expect(isCardAuthorization({ id: 'auth_1', status: 'pending' })).toBe(
      true,
    );
  });

  it('accepts an authorization with all optional fields populated', () => {
    expect(
      isCardAuthorization({
        id: 'auth_1',
        status: 'declined',
        decline_code: 'insufficient_funds',
        amount: '12.50',
        currency: 'USD',
        created_at: '2026-01-01T00:00:00Z',
      }),
    ).toBe(true);
  });

  it('rejects missing required fields', () => {
    expect(isCardAuthorization(null)).toBe(false);
    expect(isCardAuthorization({ id: 'auth_1' })).toBe(false);
    expect(isCardAuthorization({ status: 'pending' })).toBe(false);
  });

  it('rejects an optional field with the wrong type', () => {
    expect(
      isCardAuthorization({ id: 'auth_1', status: 'pending', amount: 12.5 }),
    ).toBe(false);
    expect(
      isCardAuthorization({
        id: 'auth_1',
        status: 'pending',
        decline_code: 404,
      }),
    ).toBe(false);
  });
});

describe('isCardAuthorizationsResponse', () => {
  it('accepts a valid envelope with zero or more authorizations', () => {
    expect(isCardAuthorizationsResponse({ authorizations: [], total: 0 })).toBe(
      true,
    );
    expect(
      isCardAuthorizationsResponse({
        authorizations: [{ id: 'auth_1', status: 'approved' }],
        total: 1,
      }),
    ).toBe(true);
  });

  it('rejects a response whose authorizations array contains an invalid entry', () => {
    expect(
      isCardAuthorizationsResponse({
        authorizations: [{ id: 'auth_1' }],
        total: 1,
      }),
    ).toBe(false);
  });

  it('rejects non-array authorizations or a missing/invalid total', () => {
    expect(
      isCardAuthorizationsResponse({ authorizations: 'none', total: 0 }),
    ).toBe(false);
    expect(isCardAuthorizationsResponse({ authorizations: [] })).toBe(false);
    expect(
      isCardAuthorizationsResponse({ authorizations: [], total: '0' }),
    ).toBe(false);
  });

  it('rejects non-objects', () => {
    expect(isCardAuthorizationsResponse(null)).toBe(false);
    expect(isCardAuthorizationsResponse([])).toBe(false);
  });
});
