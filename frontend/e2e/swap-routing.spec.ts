import { expect, test } from '@playwright/test';

test.describe('swap route consolidation', () => {
  test('root redirects to the production swap experience', async ({ page }) => {
    await page.goto('/');

    await expect(page).toHaveURL(/\/swap$/);
    await expect(page.getByTestId('swap-card')).toBeVisible();
    await expect(
      page.getByText(/demo swap with sell amount validation/i)
    ).toHaveCount(0);
  });

  test('header Swap navigation opens the production swap card', async ({
    page,
  }) => {
    await page.goto('/orderbook');

    await page.getByRole('link', { name: 'Swap', exact: true }).click();

    await expect(page).toHaveURL(/\/swap$/);
    await expect(page.getByTestId('swap-card')).toBeVisible();
    await expect(
      page.getByRole('link', { name: 'Swap', exact: true })
    ).toHaveAttribute('aria-current', 'page');
  });
});
