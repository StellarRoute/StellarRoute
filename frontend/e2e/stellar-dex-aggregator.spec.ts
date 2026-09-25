import { test, expect } from '@playwright/test';

/**
 * E2E coverage for the /stellar-dex-aggregator landing page.
 *
 * This is an SEO/marketing surface: it must render its FAQ for a first-time
 * visitor with no wallet installed, and must never start a live swap. These
 * tests never inject Freighter and never navigate into /swap.
 */
test.describe('Stellar DEX aggregator landing page', () => {
  test.beforeEach(async ({ page }) => {
    // Dismiss the site-wide first-run "Connect Your Wallet" dialog. This is a
    // test-only accommodation: no wallet is connected, which is what these
    // tests assert.
    await page.addInitScript(() => {
      localStorage.setItem('stellarroute.onboarding.seen', 'true');
    });
  });

  test('shows the FAQ without a wallet', async ({ page }) => {
    await page.goto('/stellar-dex-aggregator');

    await expect(
      page.getByRole('heading', { name: 'Stellar DEX aggregator FAQ', level: 2 }),
    ).toBeVisible();

    await expect(
      page.getByText('What is a Stellar DEX aggregator?'),
    ).toBeVisible();
    await expect(
      page.getByText('Does StellarRoute replace the Stellar DEX?'),
    ).toBeVisible();
    await expect(
      page.getByText('Can I combine DEX aggregation with a cross-chain swap?'),
    ).toBeVisible();
  });

  test('renders the page heading', async ({ page }) => {
    await page.goto('/stellar-dex-aggregator');

    await expect(
      page.getByRole('heading', {
        name: 'Stellar DEX aggregator for SDEX and Soroban',
        level: 1,
      }),
    ).toBeVisible();
  });

  test('requires no wallet connection', async ({ page }) => {
    await page.goto('/stellar-dex-aggregator');

    // Freighter is never injected by this spec; the FAQ must still render.
    const hasFreighter = await page.evaluate(
      () => typeof (window as unknown as Record<string, unknown>).freighterApi !== 'undefined',
    );
    expect(hasFreighter, 'test runs with no wallet present').toBe(false);

    await expect(
      page.getByRole('heading', { name: 'Stellar DEX aggregator FAQ', level: 2 }),
    ).toBeVisible();
  });

  test('starts no live swap or quote request', async ({ page }) => {
    const swapFlowRequests: string[] = [];
    await page.route('**/api/v1/**', (route) => {
      const url = route.request().url();
      if (/\/api\/v1\/(swap|quote)/.test(url)) {
        swapFlowRequests.push(url);
      }
      return route.continue();
    });

    await page.goto('/stellar-dex-aggregator');
    await expect(
      page.getByRole('heading', { name: 'Stellar DEX aggregator FAQ', level: 2 }),
    ).toBeVisible();

    expect(
      swapFlowRequests,
      'the landing page must not start a swap or quote flow',
    ).toEqual([]);

    // And it stays on its own route rather than redirecting into /swap.
    expect(new URL(page.url()).pathname).toBe('/stellar-dex-aggregator');
  });
});
