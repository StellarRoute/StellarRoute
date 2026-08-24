import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Status Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    // Mock successful calls by default
    await page.route('**/health', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'healthy',
          version: 'v1.2.3',
          timestamp: new Date().toISOString(),
          components: {
            database: 'healthy',
            indexer: 'healthy',
          },
        }),
      });
    });

    await page.route('**/health/deps', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'healthy',
          timestamp: new Date().toISOString(),
          components: {
            stellar_horizon: 'healthy',
            soroban_rpc: 'healthy',
          },
        }),
      });
    });
  });

  test('loads status page and asserts dependency rows render', async ({
    page,
  }) => {
    await page.goto('/status');

    // Check main title
    await expect(
      page.getByRole('heading', { name: 'API Status', level: 1 })
    ).toBeVisible();

    // Verify Core Components render
    await expect(page.getByText('Core Components')).toBeVisible();
    await expect(page.getByText('database')).toBeVisible();
    await expect(page.getByText('indexer')).toBeVisible();

    // Verify External Dependencies render
    await expect(page.getByText('External Dependencies')).toBeVisible();
    await expect(page.getByText('stellar horizon')).toBeVisible();
    await expect(page.getByText('soroban rpc')).toBeVisible();

    // Live health polling: toggle auto-refresh
    const autoRefreshBtn = page.getByRole('button', { name: /Auto-refresh/ });
    await expect(autoRefreshBtn).toContainText('Auto-refresh ON');
    await autoRefreshBtn.click();
    await expect(autoRefreshBtn).toContainText('Auto-refresh OFF');
  });

  test('mocked API failure shows error state', async ({ page }) => {
    // Unroute/override the successful mock for /health with a failure
    await page.route('**/health', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'internal_error',
          message: 'Failed to connect to backend',
        }),
      });
    });

    await page.goto('/status');

    // Connection Error should be shown
    await expect(page.getByText('Connection Error')).toBeVisible();
    await expect(
      page.getByText('Quote service hit an internal issue')
    ).toBeVisible();

    // Retry button is available
    const retryBtn = page.getByRole('button', { name: /retry/i });
    await expect(retryBtn).toBeVisible();
  });

  test('a11y scan with axe passes known exclusions', async ({ page }) => {
    await page.goto('/status');
    // Wait for the components to load (headers are rendered when data is loaded)
    await expect(page.getByText('Core Components')).toBeVisible();

    const results = await new AxeBuilder({ page })
      .disableRules(['color-contrast'])
      .analyze();

    const highSeverity = results.violations.filter(
      (v) => v.impact === 'critical' || v.impact === 'serious'
    );

    expect(highSeverity).toHaveLength(0);
  });
});
