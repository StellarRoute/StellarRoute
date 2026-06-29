import { test, expect } from '@playwright/test';

test.describe('Transaction History Page', () => {
  test('covers loading and empty states', async ({ page }) => {
    // Seed empty history
    await page.addInitScript(() => {
      localStorage.setItem(
        'stellar_route_tx_history_GBSU...XYZ9',
        JSON.stringify([])
      );
    });

    await page.goto('/history');

    // Assert loading state
    const loadingSkeleton = page.locator(
      '[aria-label="Loading transaction history"]'
    );
    await expect(loadingSkeleton).toBeVisible();

    // Assert transition to empty state
    await expect(loadingSkeleton).not.toBeVisible();
    await expect(page.getByText('No Transactions Found')).toBeVisible();
    await expect(
      page.getByText("You haven't made any swaps yet")
    ).toBeVisible();
  });

  test('covers populated states and features', async ({ page }) => {
    // Seed populated history
    await page.addInitScript(() => {
      const mockTxs = [
        {
          id: 'tx-1',
          hash: '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
          timestamp: Date.now() - 60000,
          fromAsset: 'XLM',
          fromAmount: '10.00',
          toAsset: 'USDC',
          toAmount: '1.23',
          exchangeRate: '0.123',
          status: 'confirmed',
        },
        {
          id: 'tx-2',
          hash: '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
          timestamp: Date.now() - 3600000,
          fromAsset: 'USDC',
          fromAmount: '5.50',
          toAsset: 'XLM',
          toAmount: '45.00',
          exchangeRate: '8.18',
          status: 'failed',
          errorMessage: 'Slippage tolerance exceeded',
        },
      ];
      localStorage.setItem(
        'stellar_route_tx_history_GBSU...XYZ9',
        JSON.stringify(mockTxs)
      );
    });

    await page.goto('/history');

    // Wait for loading to finish
    await expect(
      page.locator('[aria-label="Loading transaction history"]')
    ).not.toBeVisible();

    // Check rows render
    await expect(page.getByTestId('tx-row-tx-1')).toBeVisible();
    await expect(page.getByTestId('tx-row-tx-2')).toBeVisible();

    // Check contents
    await expect(page.getByText('-10.00')).toBeVisible();
    await expect(page.getByText('+1.23')).toBeVisible();
    await expect(page.getByText('Confirmed')).toBeVisible();

    await expect(page.getByText('-5.50')).toBeVisible();
    await expect(page.getByText('+45.00')).toBeVisible();
    await expect(page.getByText('Failed')).toBeVisible();
    await expect(page.getByText('Slippage tolerance exceeded')).toBeVisible();
  });

  test('mobile layout handles transaction lists on narrow viewports', async ({
    page,
  }) => {
    // Set viewport to mobile size
    await page.setViewportSize({ width: 375, height: 667 });

    // Seed populated history
    await page.addInitScript(() => {
      const mockTxs = [
        {
          id: 'tx-1',
          hash: '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
          timestamp: Date.now() - 60000,
          fromAsset: 'XLM',
          fromAmount: '10.00',
          toAsset: 'USDC',
          toAmount: '1.23',
          exchangeRate: '0.123',
          status: 'confirmed',
        },
      ];
      localStorage.setItem(
        'stellar_route_tx_history_GBSU...XYZ9',
        JSON.stringify(mockTxs)
      );
    });

    await page.goto('/history');

    // Assert that the page elements are visible in mobile layout
    await expect(page.getByText('Transaction History')).toBeVisible();
    await expect(page.getByTestId('tx-row-tx-1')).toBeVisible();
  });
});
