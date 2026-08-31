import { expect, test } from '@playwright/test';

/**
 * E2E coverage for the /guide page (issue #1287).
 *
 * The guide is a static, document-style page. It must load without a wallet
 * being required and must never trigger a swap prepare/submit flow.
 */
test.describe('/guide', () => {
  test('loads without a wallet and renders the heading', async ({ page }) => {
    await page.goto('/guide');

    // Page heading
    await expect(
      page.getByRole('heading', { name: /Your first live swap/i })
    ).toBeVisible();

    // Guide landing copy
    await expect(page.getByText(/A short path for traders/i)).toBeVisible();

    // All six steps are present
    await expect(page.getByText('Connect your wallet')).toBeVisible();
    await expect(page.getByText('Fund and reserve XLM')).toBeVisible();
    await expect(page.getByText('Add a trustline if needed')).toBeVisible();
    await expect(
      page.getByText('Pick a pair and enter a small amount')
    ).toBeVisible();
    await expect(
      page.getByText('Set slippage and review the route')
    ).toBeVisible();
    await expect(page.getByText('Confirm in your wallet')).toBeVisible();
  });

  test('does not prompt the user to connect a wallet', async ({ page }) => {
    await page.goto('/guide');

    // Reaching the guide must not open a wallet connect dialog / toast.
    await expect(
      page.getByRole('dialog', { name: /connect wallet|freighter/i })
    ).toHaveCount(0);

    // No wallet-connect CTA within the guide content.
    await expect(
      page.getByRole('button', { name: /connect wallet/i })
    ).toHaveCount(0);
  });

  test('never initiates a swap prepare or submit', async ({ page }) => {
    await page.goto('/guide');

    // No swap preparation / submission state may appear on the guide, since it
    // must never mount swap transaction machinery.
    await expect(
      page.getByText(/preparing swap|submitting|waiting for signature/i)
    ).toHaveCount(0);
  });

  test('renders cleanly on a mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/guide');

    await expect(
      page.getByRole('heading', { name: /Your first live swap/i })
    ).toBeVisible();
    await expect(page.getByText('Connect your wallet')).toBeVisible();
  });
});
