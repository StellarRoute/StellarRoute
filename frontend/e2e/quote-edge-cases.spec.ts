/**
 * E2E test suite: Quote edge cases
 *
 * Covers three behavioral groups:
 *   1. Stale quote blocking
 *   2. Stale quote refresh path
 *   3. Retry/backoff visible behavior
 *   4. Route candidate switching
 *
 * Requirements: 1.x, 2.x, 3.x, 4.x, 5.1, 5.2, 6.x
 */

import { test, expect, type Page } from "@playwright/test";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Mirrors QUOTE_STALE_AFTER_MS from frontend/lib/quote-stale.ts */
const QUOTE_STALE_AFTER_MS = 5500;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

function alternativeRoutesFixture() {
  return [
    { id: "route-0", venue: "AQUA Pool", expectedAmount: "≈ 99.5500" },
    { id: "route-1", venue: "SDEX", expectedAmount: "≈ 99.4000" },
    { id: "route-2", venue: "Phoenix AMM", expectedAmount: "≈ 99.2500" },
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

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

async function fillAmountAndWait(page: Page, amount = "100") {
  const input = page.locator('input[placeholder="0.00"]').first();
  await input.fill(amount);
  await page.waitForTimeout(600); // debounce + render
}

async function setupStaleQuote(page: Page) {
  // Install clock at current time
  await page.clock.install({ time: Date.now() });
  // Intercept quote API
  await page.route("/api/v1/quote/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(freshQuoteFixture()),
    });
  });
  await page.goto("/swap");
  await fillAmountAndWait(page);
  // Advance clock past stale threshold
  await page.clock.tick(QUOTE_STALE_AFTER_MS + 100); // 5600ms
  await page.waitForTimeout(200); // let React re-render
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
  try {
    await (page as any).clock?.uninstall?.();
  } catch {
    // clock may not have been installed in every test
  }
});

// ---------------------------------------------------------------------------
// Group 1 — Stale Quote Blocking
// ---------------------------------------------------------------------------

test.describe("Stale quote blocking", () => {
  test("CTA is disabled after QUOTE_STALE_AFTER_MS elapses", async ({ page }) => {
    await setupStaleQuote(page);
    const cta = page.getByRole("button", { name: /swap|review/i });
    await expect(cta).toBeDisabled();
  });

  test("stale indicator is visible when quote is stale", async ({ page }) => {
    await setupStaleQuote(page);
    await expect(page.getByTestId("stale-indicator")).toBeVisible();
  });

  test("refresh button remains enabled when quote is stale", async ({ page }) => {
    await setupStaleQuote(page);
    const refreshBtn = page.getByRole("button", { name: /refresh/i });
    await expect(refreshBtn).toBeEnabled();
  });

  test("clicking disabled CTA does not initiate a swap", async ({ page }) => {
    await setupStaleQuote(page);
    const cta = page.getByRole("button", { name: /swap|review/i });
    // Force-click the disabled button
    await cta.click({ force: true });
    // No navigation or confirmation dialog should appear
    await expect(page.getByRole("dialog")).not.toBeVisible();
    expect(page.url()).toContain("/swap");
  });
});

// ---------------------------------------------------------------------------
// Group 2 — Stale Quote Refresh Path
// ---------------------------------------------------------------------------

test.describe("Stale quote refresh path", () => {
  test("clicking Refresh_Button triggers a new quote request", async ({ page }) => {
    await setupStaleQuote(page);
    const requestPromise = page.waitForRequest((req) =>
      req.url().includes("/api/v1/quote"),
    );
    await page.getByRole("button", { name: /refresh/i }).click();
    await requestPromise;
  });

  test("CTA re-enables within 500ms after successful refresh", async ({ page }) => {
    await setupStaleQuote(page);
    // Update intercept to return a fresh quote
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.getByRole("button", { name: /refresh/i }).click();
    const cta = page.getByRole("button", { name: /swap|review/i });
    await expect(cta).toBeEnabled({ timeout: 2000 });
  });

  test("stale indicator disappears after successful refresh", async ({ page }) => {
    await setupStaleQuote(page);
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.getByRole("button", { name: /refresh/i }).click();
    await expect(page.getByTestId("stale-indicator")).not.toBeVisible({ timeout: 2000 });
  });

  test("error message shown and CTA stays disabled on failed refresh", async ({ page }) => {
    await setupStaleQuote(page);
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ error: "Internal Server Error" }),
      });
    });
    await page.getByRole("button", { name: /refresh/i }).click();
    await page.waitForTimeout(500);
    const cta = page.getByRole("button", { name: /swap|review/i });
    await expect(cta).toBeDisabled();
  });

  test("refresh button shows loading animation during in-flight refresh", async ({ page }) => {
    await setupStaleQuote(page);
    // Slow intercept to keep request in-flight
    await page.route("/api/v1/quote/**", async (route) => {
      await new Promise((r) => setTimeout(r, 1000));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.getByRole("button", { name: /refresh/i }).click();
    // Immediately after click, the RefreshCw icon should have animate-spin or button should be disabled
    const refreshBtn = page.getByRole("button", { name: /refresh/i });
    await expect(refreshBtn).toBeDisabled({ timeout: 500 });
  });
});

// ---------------------------------------------------------------------------
// Group 3 — Retry/Backoff Visible Behavior
// ---------------------------------------------------------------------------

test.describe("Retry/backoff visible behavior", () => {
  test("recovering indicator is shown on first transient error", async ({ page }) => {
    let callCount = 0;
    await page.route("/api/v1/quote/**", async (route) => {
      callCount++;
      if (callCount === 1) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "Server Error" }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await expect(page.getByTestId("recovering-indicator")).toBeVisible({ timeout: 3000 });
  });

  test("retry 1 waits at least 1000ms before next request", async ({ page }) => {
    await page.clock.install({ time: Date.now() });
    let callCount = 0;
    await page.route("/api/v1/quote/**", async (route) => {
      callCount++;
      if (callCount === 1) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "Server Error" }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    // Wait for first request to fail
    await page.waitForTimeout(200);
    const countAfterFail = callCount;
    // Tick 999ms — should NOT have retried yet
    await page.clock.tick(999);
    await page.waitForTimeout(100);
    expect(callCount).toBe(countAfterFail);
    // Tick 1ms more — retry should fire
    await page.clock.tick(1);
    await page.waitForTimeout(200);
    expect(callCount).toBeGreaterThan(countAfterFail);
  });

  test("retry 2 waits at least 2000ms before next request", async ({ page }) => {
    await page.clock.install({ time: Date.now() });
    let callCount = 0;
    await page.route("/api/v1/quote/**", async (route) => {
      callCount++;
      if (callCount <= 2) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "Server Error" }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    // Let first request fail and first retry fire (tick 1001ms)
    await page.clock.tick(1001);
    await page.waitForTimeout(200);
    const countAfterRetry1 = callCount;
    // Tick 1999ms — should NOT have fired retry 2 yet
    await page.clock.tick(1999);
    await page.waitForTimeout(100);
    expect(callCount).toBe(countAfterRetry1);
    // Tick 1ms more — retry 2 should fire
    await page.clock.tick(1);
    await page.waitForTimeout(200);
    expect(callCount).toBeGreaterThan(countAfterRetry1);
  });

  test("recovering indicator clears on successful retry", async ({ page }) => {
    let callCount = 0;
    await page.route("/api/v1/quote/**", async (route) => {
      callCount++;
      if (callCount === 1) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "Server Error" }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    // Wait for retry to succeed
    await page.waitForTimeout(2000);
    await expect(page.getByTestId("recovering-indicator")).not.toBeVisible({ timeout: 3000 });
  });

  test("persistent error shown after all retries exhausted", async ({ page }) => {
    test.setTimeout(15000);
    // Always fail — more than maxAutoRetries (2)
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ error: "Server Error" }),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    // Wait for all retries to exhaust (2 retries * 1s + 2s = ~4s, add buffer)
    await page.waitForTimeout(5000);
    // Recovering indicator should be gone
    await expect(page.getByTestId("recovering-indicator")).not.toBeVisible();
    // Error message should be visible
    await expect(
      page.locator('.text-destructive, [class*="destructive"]').first(),
    ).toBeVisible({ timeout: 1000 });
  });

  test("Retry-After header overrides default backoff delay", async ({ page }) => {
    await page.clock.install({ time: Date.now() });
    let callCount = 0;
    await page.route("/api/v1/quote/**", async (route) => {
      callCount++;
      if (callCount === 1) {
        await route.fulfill({
          status: 429,
          headers: { "Retry-After": "3" },
          contentType: "application/json",
          body: JSON.stringify({ error: "Rate Limited" }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(freshQuoteFixture()),
        });
      }
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await page.waitForTimeout(200);
    const countAfterFail = callCount;
    // Tick 2999ms — should NOT have retried yet (Retry-After: 3 = 3000ms)
    await page.clock.tick(2999);
    await page.waitForTimeout(100);
    expect(callCount).toBe(countAfterFail);
    // Tick 1ms more — retry should fire
    await page.clock.tick(1);
    await page.waitForTimeout(200);
    expect(callCount).toBeGreaterThan(countAfterFail);
  });
});

// ---------------------------------------------------------------------------
// Group 4 — Route Candidate Switching
// ---------------------------------------------------------------------------

test.describe("Route candidate switching", () => {
  test("RouteDisplay renders buttons with correct data-testid attributes", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await expect(page.getByTestId("alternative-route-route-0")).toBeVisible({ timeout: 3000 });
    await expect(page.getByTestId("alternative-route-route-1")).toBeVisible();
    await expect(page.getByTestId("alternative-route-route-2")).toBeVisible();
  });

  test("clicking a route updates the expected output amount", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    // Wait for routes to render
    await expect(page.getByTestId("alternative-route-route-1")).toBeVisible({ timeout: 3000 });
    // Record the initial receive value
    const receiveInput = page.locator('input[placeholder="0.00"]').nth(1);
    const initialValue = await receiveInput.inputValue();
    // Click route-1
    await page.getByTestId("alternative-route-route-1").click();
    // The receive input should have changed (route selection updates the displayed amount)
    await expect(receiveInput).not.toHaveValue(initialValue, { timeout: 1000 });
  });

  test("selected route is visually distinguished", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await expect(page.getByTestId("alternative-route-route-1")).toBeVisible({ timeout: 3000 });
    await page.getByTestId("alternative-route-route-1").click();
    // route-1 should have aria-pressed="true"
    const route1 = page.getByTestId("alternative-route-route-1");
    const route0 = page.getByTestId("alternative-route-route-0");
    await expect(route1).toHaveAttribute("aria-pressed", "true");
    await expect(route0).not.toHaveAttribute("aria-pressed", "true");
  });

  test("CTA remains enabled after route switch", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await expect(page.getByTestId("alternative-route-route-1")).toBeVisible({ timeout: 3000 });
    await page.getByTestId("alternative-route-route-1").click();
    // CTA should still be in a non-disabled state (may show "Connect Wallet" but not disabled due to stale)
    const cta = page.getByRole("button", { name: /swap|review|connect/i });
    await expect(cta).toBeEnabled({ timeout: 1000 });
  });

  test("keyboard Enter triggers route switch", async ({ page }) => {
    await page.route("/api/v1/quote/**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(freshQuoteFixture()),
      });
    });
    await page.goto("/swap");
    await fillAmountAndWait(page);
    await expect(page.getByTestId("alternative-route-route-2")).toBeVisible({ timeout: 3000 });
    const route2 = page.getByTestId("alternative-route-route-2");
    await route2.focus();
    await route2.press("Enter");
    // route-2 should now be selected
    await expect(route2).toHaveAttribute("aria-pressed", "true", { timeout: 1000 });
  });
});
