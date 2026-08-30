import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
  test('settings page is reachable directly by URL and renders all sections', async ({
    page,
  }) => {
    await page.goto('/settings');

    // Page title heading
    await expect(
      page.getByRole('heading', { name: /Settings/i, level: 1 })
    ).toBeVisible();

    // Key settings sections
    await expect(page.getByText('Trade Settings')).toBeVisible();
    await expect(page.getByText('Appearance')).toBeVisible();
    await expect(page.getByText('Accessibility')).toBeVisible();
    await expect(page.getByText('Browser Notifications')).toBeVisible();
    await expect(page.getByText('Reset Settings')).toBeVisible();
  });

  test('toggles theme and persists reload', async ({ page }) => {
    await page.goto('/settings');

    // Clear settings to start clean
    await page.evaluate(() =>
      localStorage.removeItem('stellar_route_settings')
    );
    await page.reload();

    // Find the theme select trigger (combobox)
    const selectTrigger = page.getByRole('combobox').first();
    await expect(selectTrigger).toBeVisible();

    // Click to open the select options
    await selectTrigger.click();

    // Click on the 'Dark' option
    const darkOption = page.getByRole('option', { name: /dark/i });
    await darkOption.click();

    // Assert theme was updated on document element (HTML) class list
    await expect(page.locator('html')).toHaveClass(/dark/);

    // Reload page
    await page.reload();

    // Assert theme persists after reload
    await expect(page.locator('html')).toHaveClass(/dark/);
  });

  test('updates slippage tolerance and persists across reload', async ({ page }) => {
    await page.goto('/settings');

    // Find slippage input
    const slippageInput = page.locator('input[type="number"]').first();
    await expect(slippageInput).toBeVisible();

    // Fill with new value
    await slippageInput.fill('1.5');
    await slippageInput.blur();

    // Reload to verify persistence in localStorage
    await page.reload();
    const reloadedInput = page.locator('input[type="number"]').first();
    await expect(reloadedInput).toHaveValue('1.5');
  });

  test('resets settings to defaults when reset button is clicked', async ({ page }) => {
    await page.goto('/settings');

    // Modify slippage first
    const slippageInput = page.locator('input[type="number"]').first();
    await slippageInput.fill('2.5');
    await slippageInput.blur();
    await expect(slippageInput).toHaveValue('2.5');

    // Click Reset to Defaults button
    const resetButton = page.getByRole('button', { name: /Reset to Defaults/i });
    await expect(resetButton).toBeVisible();
    await resetButton.click();

    // Slippage should reset to default (0.5)
    await expect(slippageInput).toHaveValue('0.5');
  });

  test('renders cleanly on mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/settings');

    await expect(
      page.getByRole('heading', { name: /Settings/i, level: 1 })
    ).toBeVisible();
    await expect(page.getByText('Trade Settings')).toBeVisible();
    await expect(page.getByText('Appearance')).toBeVisible();
  });
});
