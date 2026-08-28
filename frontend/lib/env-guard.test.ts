import { describe, expect, it } from 'vitest';
import {
  assertFrontendProductionEnv,
  enforceFrontendProductionEnv,
  isProductionFrontendEnv,
  resolveCriticalApiUrl,
} from './env-guard';

describe('env-guard', () => {
  it('does not treat plain development as production', () => {
    expect(
      isProductionFrontendEnv({
        NODE_ENV: 'development',
        VERCEL_ENV: 'development',
      })
    ).toBe(false);
  });

  it('treats VERCEL_ENV=production as production', () => {
    expect(
      isProductionFrontendEnv({
        NODE_ENV: 'production',
        VERCEL_ENV: 'production',
      })
    ).toBe(true);
  });

  it('treats STELLARROUTE_ENV=production as production', () => {
    expect(
      isProductionFrontendEnv({
        NODE_ENV: 'development',
        STELLARROUTE_ENV: 'production',
      })
    ).toBe(true);
  });

  it('allows missing API URL in development', () => {
    expect(
      assertFrontendProductionEnv({
        NODE_ENV: 'development',
      })
    ).toEqual({ ok: true });
  });

  it('fails production when API URL is missing', () => {
    const result = assertFrontendProductionEnv({
      VERCEL_ENV: 'production',
      NODE_ENV: 'production',
    });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.message).toMatch(/NEXT_PUBLIC_API_URL/);
    }
  });

  it('fails production when API URL is localhost', () => {
    const result = assertFrontendProductionEnv({
      VERCEL_ENV: 'production',
      NODE_ENV: 'production',
      NEXT_PUBLIC_API_URL: 'http://localhost:8080/api/v1',
    });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.message).toMatch(/localhost/);
    }
  });

  it('passes production with a public HTTPS API URL', () => {
    expect(
      assertFrontendProductionEnv({
        VERCEL_ENV: 'production',
        NODE_ENV: 'production',
        NEXT_PUBLIC_API_URL: 'https://api.example.com/api/v1',
        NEXT_PUBLIC_STELLAR_NETWORK: 'testnet',
      })
    ).toEqual({ ok: true });
  });

  it('prefers NEXT_PUBLIC_API_URL_TESTNET for testnet', () => {
    expect(
      resolveCriticalApiUrl({
        NEXT_PUBLIC_STELLAR_NETWORK: 'testnet',
        NEXT_PUBLIC_API_URL_TESTNET: 'https://api-test.example.com',
        NEXT_PUBLIC_API_URL: 'https://api-shared.example.com',
      })
    ).toBe('https://api-test.example.com');
  });

  it('enforceFrontendProductionEnv throws on bad production config', () => {
    expect(() =>
      enforceFrontendProductionEnv({
        VERCEL_ENV: 'production',
        NEXT_PUBLIC_API_URL: 'http://127.0.0.1:3000',
      })
    ).toThrow(/env-guard/);
  });
});
