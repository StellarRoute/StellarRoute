import { test, expect } from '@playwright/test';

/**
 * E2E coverage for the /docs surface.
 *
 * The docs page is a static index of links — it must render for a visitor who
 * has no wallet extension installed and has never connected one. These tests
 * therefore never stub a wallet, never navigate to /swap, and never trigger a
 * quote or swap request.
 */
test.describe('Docs page', () => {
  test.beforeEach(async ({ page }) => {
    // The site-wide wallet button opens a first-run "Connect Your Wallet"
    // dialog for any visitor who has not seen it, which overlays the page.
    // Marking it seen is a test-only accommodation — the visitor still has no
    // wallet connected, which is exactly what these tests assert.
    await page.addInitScript(() => {
      localStorage.setItem('stellarroute.onboarding.seen', 'true');
    });
  });

  test('loads without a wallet and shows the heading', async ({ page }) => {
    // Fail loudly if the docs surface ever starts calling swap/quote APIs:
    // this page must stay renderable with no backend and no wallet.
    const swapFlowRequests: string[] = [];
    await page.route('**/api/v1/**', (route) => {
      const url = route.request().url();
      if (/\/api\/v1\/(swap|quote)/.test(url)) {
        swapFlowRequests.push(url);
      }
      return route.continue();
    });

    await page.goto('/docs');

    await expect(
      page.getByRole('heading', { name: 'StellarRoute Docs', level: 1 }),
    ).toBeVisible();

    expect(
      swapFlowRequests,
      'the docs page must not start a swap or quote flow',
    ).toEqual([]);
  });

  test('does not navigate to /swap', async ({ page }) => {
    await page.goto('/docs');

    await expect(
      page.getByRole('heading', { name: 'StellarRoute Docs', level: 1 }),
    ).toBeVisible();

    // The page under test stays on /docs; no redirect into the swap flow.
    expect(new URL(page.url()).pathname).toBe('/docs');
  });

  test('renders the documentation index entries', async ({ page }) => {
    await page.goto('/docs');

    // Scoped to <main>: some of these titles also appear in the site nav, so
    // an unscoped text match is ambiguous under Playwright strict mode.
    const index = page.getByRole('main');

    await expect(
      index.getByRole('link', { name: /First Live Swap Guide/ }),
    ).toBeVisible();
    await expect(
      index.getByRole('link', { name: /API Reference/ }),
    ).toBeVisible();
    await expect(
      index.getByRole('link', { name: /Developer Guide/ }),
    ).toBeVisible();
  });

  test('does not require a wallet connection', async ({ page }) => {
    // No wallet is injected into the page context at all — if the docs surface
    // gated on one, the heading below would never appear.
    await page.goto('/docs');

    const hasFreighter = await page.evaluate(
      () => typeof (window as unknown as Record<string, unknown>).freighterApi !== 'undefined',
    );
    expect(hasFreighter, 'test runs with no wallet present').toBe(false);

    await expect(
      page.getByRole('heading', { name: 'StellarRoute Docs', level: 1 }),
    ).toBeVisible();
  });
});
