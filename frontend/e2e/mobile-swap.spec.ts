import { test, expect, type Page } from '@playwright/test';

/**
 * Mobile smoke for swap confirm + wallet connect (#1006).
 * Verify: npm --prefix frontend run test:e2e -- mobile-swap
 */

async function gotoSwap(page: Page) {
  await page.goto('/swap');
  await page.waitForSelector(
    '[data-testid="swap-card"], form, [class*="Card"]',
    { timeout: 15_000 }
  );
}

test.describe('mobile-swap smoke @375px', () => {
  test.use({ viewport: { width: 375, height: 812 } });

  test('wallet connect dialog fits viewport without clipping', async ({
    page,
  }) => {
    await gotoSwap(page);

    const connectBtn = page.getByRole('button', { name: /connect wallet/i }).first();
    await expect(connectBtn).toBeVisible();
    await connectBtn.click();

    const dialog = page.getByTestId('wallet-connect-dialog');
    await expect(dialog).toBeVisible({ timeout: 5_000 });

    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.width).toBeLessThanOrEqual(375);
      expect(box.height).toBeLessThanOrEqual(812);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.y).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(375 + 1);
      expect(box.y + box.height).toBeLessThanOrEqual(812 + 1);
    }

    await expect(dialog.getByText(/connect your wallet/i)).toBeVisible();
  });

  test('swap confirm modal is usable at 375px when opened', async ({
    page,
  }) => {
    await gotoSwap(page);

    // Best-effort: open review modal if the CTA is available in this env.
    const payInput = page.locator('input[placeholder="0.00"]').first();
    if (await payInput.isVisible().catch(() => false)) {
      await payInput.fill('1');
      await page.waitForTimeout(500);
    }

    const swapCta = page
      .locator(
        'button:has-text("Review"), button:has-text("Swap"), button[type="submit"]'
      )
      .first();
    if (await swapCta.isVisible().catch(() => false)) {
      await swapCta.click({ trial: true }).catch(() => undefined);
      await swapCta.click().catch(() => undefined);
    }

    const dialog = page.getByTestId('swap-confirm-dialog');
    const opened = await dialog
      .waitFor({ state: 'visible', timeout: 3_000 })
      .then(() => true)
      .catch(() => false);

    if (!opened) {
      // Still assert the page itself does not overflow at 375px.
      const scrollWidth = await page.evaluate(() => document.body.scrollWidth);
      const clientWidth = await page.evaluate(() => document.body.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth);
      return;
    }

    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.width).toBeLessThanOrEqual(375);
      expect(box.height).toBeLessThanOrEqual(812);
      expect(box.width / 375).toBeGreaterThanOrEqual(0.85);
    }

    const confirm = dialog.getByRole('button', { name: /confirm swap/i });
    if (await confirm.isVisible().catch(() => false)) {
      const btnBox = await confirm.boundingBox();
      expect(btnBox?.height ?? 0).toBeGreaterThanOrEqual(48);
    }
  });
});
