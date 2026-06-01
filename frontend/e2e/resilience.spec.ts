import { test, expect, type Page } from "@playwright/test";

function alternativeRoutesFixture() {
  return [
    { id: "route-0", venue: "AQUA Pool", expectedAmount: "≈ 99.5500" },
    { id: "route-1", venue: "SDEX", expectedAmount: "≈ 99.4000" },
  ];
}

function freshQuoteFixture() {
  return {
    base_asset: { asset_type: "native" },
    quote_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
    },
    amount: "100",
    price: "0.995",
    total: "99.5",
    price_impact: "0.1",
    quote_type: "sell",
    path: [],
    timestamp: Math.floor(Date.now() / 1000),
    source_timestamp: Date.now(),
    alternativeRoutes: alternativeRoutesFixture(),
  };
}

async function fillAmountAndWait(page: Page, amount = "100") {
  const input = page.getByTestId("amount-input").first();
  await input.fill(amount);
  await page.waitForTimeout(2000); 
}

test.describe("Frontend resilience - Flaky network and latency spikes", () => {
  test.afterEach(async ({ page }) => {
    await page.unroute("**");
  });

  test("app handles extreme latency spikes gracefully", async ({ page }) => {
    test.setTimeout(60000);

    // Intercept quote API to be slow (6 seconds)
    await page.route("**/api/v1/quote*", async (route) => {
      await new Promise(resolve => setTimeout(resolve, 6000));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });

    await page.goto("/swap");
    await fillAmountAndWait(page, "100");

    // Check for recovery - should eventually show routes
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 25000 });
  });

  test("app handles packet loss (aborted requests) and recovers", async ({ page }) => {
    test.setTimeout(60000);
    let requestCount = 0;
    await page.route("**/api/v1/quote*", async (route) => {
      requestCount++;
      if (requestCount <= 2) {
        await route.abort("failed");
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });

    await page.goto("/swap");
    await fillAmountAndWait(page, "100");

    // Recovery check
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 25000 });
  });

  test("app handles server 504 timeouts (Gateway Timeout)", async ({ page }) => {
    test.setTimeout(60000);
    await page.route("**/api/v1/quote*", async (route) => {
      await route.fulfill({
        status: 504,
        contentType: "application/json",
        body: JSON.stringify({ error: "Gateway Timeout" }),
      });
    });

    await page.goto("/swap");
    await fillAmountAndWait(page, "100");

    // Should show error message in the UI
    // We look for any text that indicates an error
    await expect(page.locator('p.text-destructive, [data-testid="error-message"]')).toBeVisible({ timeout: 20000 });
  });

  test("core actions remain recoverable via manual refresh after network drops", async ({ page }) => {
    test.setTimeout(60000);
    let networkUp = false;
    await page.route("**/api/v1/quote*", async (route) => {
      if (!networkUp) {
        await route.abort("internetdisconnected");
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });

    await page.goto("/swap");
    await fillAmountAndWait(page, "100");

    // Wait for failure state (error message visible)
    await expect(page.locator('p.text-destructive')).toBeVisible({ timeout: 15000 });

    // "Restore" network
    networkUp = true;

    // Manual refresh click
    const refreshBtn = page.getByRole("button", { name: /refresh/i });
    await refreshBtn.click();

    // Should recover
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 10000 });
    
    // Ensure CTA is active again
    const cta = page.getByRole("button", { name: /swap|review|connect/i });
    await expect(cta).toBeEnabled();
  });
});
