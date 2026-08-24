import { afterEach, describe, expect, it, vi } from 'vitest';
import { withTimeout } from './detect';

describe('withTimeout', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('is SSR-safe when window is undefined', async () => {
    vi.stubGlobal('window', undefined);

    await expect(
      withTimeout(Promise.resolve('ok'), 50, 'fallback')
    ).resolves.toBe('ok');

    await expect(
      withTimeout(
        new Promise<string>((resolve) => {
          globalThis.setTimeout(() => resolve('late'), 200);
        }),
        20,
        'fallback'
      )
    ).resolves.toBe('fallback');
  });

  it('returns fallback when the promise rejects', async () => {
    await expect(
      withTimeout(Promise.reject(new Error('boom')), 50, 'fallback')
    ).resolves.toBe('fallback');
  });
});
