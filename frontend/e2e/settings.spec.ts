import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
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
});
