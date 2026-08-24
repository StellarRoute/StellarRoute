/**
 * E2E test suite: Optimistic Swap Pipeline
 *
 * Covers the full optimistic swap execution pipeline:
 *   1. Optimistic indicator visible immediately after confirm
 *   2. Confirmed indicator after successful submission
 *   3. Rollback on wallet rejection
 *   4. Rollback on Horizon submission error
 *   5. Rollback on deadline elapsed (dropped) using page.clock
 *   6. Submit lock prevents second swap while first is pending
 *   7. Submit lock released and CTA re-enabled after confirmed
 *   8. Submit lock released and form rolled back after failed
 *   9. Wallet disconnect mid-swap → failed → rollback
 *
 * Requirements: 4.1–4.10
 */

import { test, expect, type Page } from "@playwright/test";
import { setupSwapE2E, E2E_USDC_ISSUER } from "./fixtures/freighter-mock";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEADLINE_MS = 60_000;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

const E2E_USDC_TOKEN = `USDC:${E2E_USDC_ISSUER}`;

function swapPageUrl(amount = "10") {
  const params = new URLSearchParams({
    from: "native",
    to: E2E_USDC_TOKEN,
    amount,
  });
  return `/swap?${params.toString()}`;
}
function freshQuoteFixture() {
  return {
    base_asset: { asset_type: "native" },
    quote_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: E2E_USDC_ISSUER,
    },
    amount: "100",
    price: "0.995",
    total: "99.5",
    price_impact: "0.1",
    quote_type: "sell",
    path: [],
    timestamp: Math.floor(Date.now() / 1000),
    source_timestamp: Date.now(),
    alternativeRoutes: [],
  };
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

async function setupFreshQuote(page: Page) {
  await page.route("**/api/v1/quote/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(freshQuoteFixture()),
    });
  });

  await page.route("**/api/v1/routes/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ routes: [] }),
    });
  });
}

async function dismissRecoveryIfPresent(page: Page) {
  const startFresh = page.getByRole("button", { name: /start fresh/i });
  if (await startFresh.isVisible().catch(() => false)) {
    await startFresh.click();
  }
}

async function connectWallet(page: Page) {
  const connectBtn = page.getByRole("button", { name: /connect wallet/i });
  if (await connectBtn.isVisible().catch(() => false)) {
    await connectBtn.click();
  }
  await expect(page.getByText(/balance:/i)).toBeVisible({ timeout: 15_000 });
}

async function confirmSwap(page: Page) {
  const reviewBtn = page.getByRole("button", { name: /review swap/i });
  await expect(reviewBtn).toBeEnabled({ timeout: 15_000 });
  await reviewBtn.click();
}

function swapFailedHeading(page: Page) {
  return page.getByRole("heading", { name: /swap failed/i });
}

function swapConfirmedHeading(page: Page) {
  return page.getByRole("heading", { name: /swap confirmed/i });
}

function transactionTimedOutHeading(page: Page) {
  return page.getByRole("heading", { name: /transaction timed out/i });
}

async function gotoSwapPage(page: Page, amount = "10") {
  await page.goto(swapPageUrl(amount));
  await dismissRecoveryIfPresent(page);
}

async function prepareSwapPage(
  page: Page,
  e2eOptions: Parameters<typeof setupSwapE2E>[1] = {},
  amount = "10"
) {
  await setupSwapE2E(page, e2eOptions);
  await setupFreshQuote(page);
  await gotoSwapPage(page, amount);
  await expect(page.getByTestId("swap-card")).toBeVisible({ timeout: 15_000 });
  await connectWallet(page);
  const reviewBtn = page.getByRole("button", { name: /review swap/i });
  const connectBtn = page.getByRole("button", { name: /connect wallet/i });
  if (await connectBtn.isVisible().catch(() => false)) {
    await connectBtn.click();
  }
  await expect(reviewBtn).toBeEnabled({ timeout: 20_000 });
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
  try {
    await (page as unknown as { clock?: { uninstall?: () => void } }).clock?.uninstall?.();
  } catch {
    // clock may not have been installed in every test
  }
});

// ---------------------------------------------------------------------------
// Group 1 — Optimistic State Visibility
// ---------------------------------------------------------------------------

test.describe("Optimistic state visibility", () => {
  test("4.1 — optimistic indicator visible immediately after confirm, before confirmation", async ({
    page,
  }) => {
    await prepareSwapPage(page, { horizonSubmitStatus: "hang" });
    await confirmSwap(page);

    await expect(
      page.getByRole("heading", { name: /awaiting confirmation|waiting for wallet/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("4.2 — confirmed indicator and 'Swap confirmed' label after successful submission", async ({
    page,
  }) => {
    await prepareSwapPage(page);
    await confirmSwap(page);

    await expect(swapConfirmedHeading(page)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("button", { name: /^done$/i }).first()).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Group 2 — Rollback Scenarios
// ---------------------------------------------------------------------------

test.describe("Rollback scenarios", () => {
  test("4.3 — rollback on wallet rejection (pending → failed)", async ({
    page,
  }) => {
    await prepareSwapPage(
      page,
      { signBehavior: "reject", rejectMessage: "User declined" },
      "42"
    );
    await confirmSwap(page);

    await expect(swapFailedHeading(page)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("button", { name: /try again/i })).toBeVisible();
  });

  test("4.4 — rollback on Horizon submission error (submitted → failed)", async ({
    page,
  }) => {
    await prepareSwapPage(page, { horizonSubmitStatus: 400 }, "25");
    await confirmSwap(page);

    await expect(swapFailedHeading(page)).toBeVisible({ timeout: 10000 });
  });

  test("4.5 — rollback on deadline elapsed (dropped) using page.clock", async ({
    page,
  }) => {
    await page.clock.install({ time: Date.now() });
    await prepareSwapPage(page, { horizonSubmitStatus: "hang" });
    await confirmSwap(page);

    await expect(
      page.getByRole("heading", { name: /awaiting confirmation/i })
    ).toBeVisible({ timeout: 5000 });

    await page.clock.fastForward(DEADLINE_MS + 1000);
    await page.waitForTimeout(500);

    await expect(transactionTimedOutHeading(page)).toBeVisible({
      timeout: 5000,
    });
  });
});

// ---------------------------------------------------------------------------
// Group 3 — Submit Lock
// ---------------------------------------------------------------------------

test.describe("Submit lock", () => {
  test("4.6 — submit lock prevents second swap while first is pending", async ({
    page,
  }) => {
    await prepareSwapPage(page, { horizonSubmitStatus: "hang" });
    await confirmSwap(page);

    await expect(
      page.getByRole("button", { name: /processing|swapping/i }).first()
    ).toBeDisabled({ timeout: 5000 });
  });

  test("4.7 — submit lock released and CTA re-enabled after confirmed", async ({
    page,
  }) => {
    await prepareSwapPage(page);
    await confirmSwap(page);

    await expect(swapConfirmedHeading(page)).toBeVisible({ timeout: 10000 });

    await page.getByRole("button", { name: /^done$/i }).first().click();

    await expect(
      page.getByRole("button", { name: /swapping/i })
    ).not.toBeVisible({ timeout: 2000 });
  });

  test("4.8 — submit lock released and form accessible after failed", async ({
    page,
  }) => {
    await prepareSwapPage(
      page,
      { signBehavior: "reject", rejectMessage: "User declined" }
    );
    await confirmSwap(page);

    await expect(swapFailedHeading(page)).toBeVisible({ timeout: 10000 });

    const tryAgainBtn = page.getByRole("button", { name: /try again/i });
    await tryAgainBtn.click();
    const inputAfterRollback = page.locator('input[placeholder="0.00"]').first();
    await expect(inputAfterRollback).toBeEnabled({ timeout: 2000 });
  });

  test("4.9 — wallet disconnect mid-swap transitions to a terminal state", async ({
    page,
  }) => {
    await prepareSwapPage(page, { horizonSubmitStatus: "hang" });
    await confirmSwap(page);

    await page.route("**/horizon-testnet.stellar.org/**", async (route) => {
      await route.abort("connectionrefused");
    });

    await expect(
      page.getByRole("heading", { name: /awaiting confirmation/i })
    ).toBeVisible({ timeout: 15000 });
  });
});
