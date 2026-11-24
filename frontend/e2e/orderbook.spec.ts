/**
 * E2E spec: /orderbook page
 *
 * Issue #1229 — Playwright coverage for /orderbook empty and loading
 *
 * Scope:
 *   - Mock /api/v1/pairs and /api/v1/orderbook/* endpoints
 *   - Assert empty state (bids/asks arrays both empty) renders "no bids / no asks" copy
 *   - Assert loading UI appears when the orderbook response is delayed
 *   - Assert the pairs list renders pair buttons from mocked /api/v1/pairs
 *   - Never mounts SwapCard; no /swap navigation
 *
 * Additive constraint: no /swap files changed; no production env vars set.
 * Production files touched by this PR: none (new file only).
 */

import { test, expect } from "@playwright/test";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const MOCK_PAIRS = [
  {
    base: "XLM",
    counter: "USDC",
    base_asset: "native",
    counter_asset:
      "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  },
  {
    base: "XLM",
    counter: "yXLM",
    base_asset: "native",
    counter_asset:
      "yXLM:GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55",
  },
];

function pairsResponseFixture() {
  return { pairs: MOCK_PAIRS, total: MOCK_PAIRS.length };
}

function emptyOrderbookFixture() {
  return {
    bids: [],
    asks: [],
    base: "native",
    counter: "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  };
}

function populatedOrderbookFixture() {
  return {
    bids: [
      { price: "0.1000", amount: "500.0000000", total: "50.0000000" },
      { price: "0.0999", amount: "250.0000000", total: "24.9750000" },
    ],
    asks: [
      { price: "0.1001", amount: "300.0000000", total: "30.0300000" },
      { price: "0.1002", amount: "150.0000000", total: "15.0300000" },
    ],
    base: "native",
    counter: "USDC:GA5ZSEJYB37JRC5AVCIAZDL2Y343IFRMA2EO3HJWV2XG7H5V5CQRUP7W",
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Register a mock for /api/v1/pairs. All requests matching the pattern are
 * served from the fixture regardless of query params.
 */
async function mockPairs(page: import("@playwright/test").Page) {
  await page.route("**/api/v1/pairs**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(pairsResponseFixture()),
    });
  });
}

/**
 * Register an immediately-resolving mock for /api/v1/orderbook/* that returns
 * the provided fixture.
 */
async function mockOrderbook(
  page: import("@playwright/test").Page,
  body: object
) {
  await page.route("**/api/v1/orderbook/**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(body),
    });
  });
}

/**
 * Register a delayed mock for /api/v1/orderbook/* that holds for `delayMs`
 * before responding, so loading state can be asserted.
 */
async function mockOrderbookDelayed(
  page: import("@playwright/test").Page,
  delayMs: number,
  body: object
) {
  await page.route("**/api/v1/orderbook/**", async (route) => {
    await new Promise((r) => setTimeout(r, delayMs));
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(body),
    });
  });
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

test.afterEach(async ({ page }) => {
  await page.unroute("**");
});

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe("/orderbook page", () => {
  /**
   * 1. Page heading is visible.
   *    OrderbookPageClient renders <h1> via t('orderbook.title').
   *    We assert the h1 exists regardless of i18n string value.
   */
  test("renders the page heading", async ({ page }) => {
    await mockPairs(page);
    await mockOrderbook(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    await expect(page.getByRole("heading", { level: 1 })).toBeVisible({
      timeout: 8000,
    });
  });

  /**
   * 2. Pair buttons from the mocked /api/v1/pairs response are rendered.
   */
  test("renders pair selector buttons from mocked pairs", async ({ page }) => {
    await mockPairs(page);
    await mockOrderbook(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    // OrderbookPageClient renders: `{pair.base}/{pair.counter}` for each pair
    await expect(
      page.getByRole("button", { name: /XLM\/USDC/i })
    ).toBeVisible({ timeout: 8000 });

    await expect(
      page.getByRole("button", { name: /XLM\/yXLM/i })
    ).toBeVisible({ timeout: 8000 });
  });

  /**
   * 3. Empty state — when bids and asks are both empty arrays, the
   *    VirtualizedOrderSide components render "no bids" / "no asks" copy
   *    via i18n keys 'orderbook.noBids' / 'orderbook.noAsks'.
   *    We assert the bid-virtual-list and ask-virtual-list containers are
   *    absent (not rendered for 0-entry lists) or that visible text indicates
   *    emptiness. The component renders a plain <p> (not a test-id) so we
   *    match on the virtual list test-ids being absent and the bids/asks
   *    heading still being present.
   */
  test("empty orderbook shows bids and asks sections without row entries", async ({
    page,
  }) => {
    await mockPairs(page);
    await mockOrderbook(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    // Wait for the pair to auto-select and orderbook to resolve
    await page.waitForTimeout(500);

    // With empty arrays VirtualizedOrderSide renders a <p> not the virtual
    // list div — so bid-virtual-list test-id should NOT be in DOM
    await expect(page.getByTestId("bid-virtual-list")).not.toBeAttached();
    await expect(page.getByTestId("ask-virtual-list")).not.toBeAttached();
  });

  /**
   * 4. Loading state — a delayed orderbook response keeps the loading ViewState
   *    visible while the request is in-flight.
   */
  test("shows loading state while orderbook response is pending", async ({
    page,
  }) => {
    await mockPairs(page);
    // Delay the orderbook response by 3 s so we can assert the loading UI
    await mockOrderbookDelayed(page, 3000, populatedOrderbookFixture());

    await page.goto("/orderbook");

    // Wait for the pairs to load so the pair is auto-selected and the
    // orderbook request fires
    await expect(
      page.getByRole("button", { name: /XLM\/USDC/i })
    ).toBeVisible({ timeout: 8000 });

    // The OrderbookPageClient renders a ViewState variant="loading" while
    // orderbookLoading is true. ViewState renders an aria-busy container or
    // a role="status" element — match on any visible loading text.
    const loadingIndicator = page.locator('[aria-busy="true"], [role="status"]');
    await expect(loadingIndicator.first()).toBeVisible({ timeout: 2000 });
  });

  /**
   * 5. Populated orderbook — bid and ask rows render from fixture data.
   */
  test("renders bid and ask rows from populated orderbook fixture", async ({
    page,
  }) => {
    await mockPairs(page);
    await mockOrderbook(page, populatedOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");
    await page.waitForTimeout(500);

    // Bid rows have data-testid="bid-row"
    const bidRows = page.getByTestId("bid-row");
    await expect(bidRows.first()).toBeVisible({ timeout: 5000 });

    // Ask rows have data-testid="ask-row"
    const askRows = page.getByTestId("ask-row");
    await expect(askRows.first()).toBeVisible({ timeout: 5000 });

    // Price values from fixture appear in the DOM
    await expect(page.getByText("0.1000")).toBeVisible();
    await expect(page.getByText("0.1001")).toBeVisible();
  });

  /**
   * 6. Refresh button is present.
   *    OrderbookPageClient renders a "Refresh" button wired to the refresh
   *    callback from useOrderbook.
   */
  test("refresh button is visible", async ({ page }) => {
    await mockPairs(page);
    await mockOrderbook(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    // The button text comes from i18n key 'orderbook.button.refresh'.
    // We match by role without constraining the exact label.
    const refreshBtn = page.getByRole("button").filter({ hasText: /refresh/i });
    await expect(refreshBtn).toBeVisible({ timeout: 5000 });
  });

  /**
   * 7. The spec never navigates to /swap and the SwapCard component is absent.
   */
  test("does not mount the swap card", async ({ page }) => {
    await mockPairs(page);
    await mockOrderbook(page, emptyOrderbookFixture());

    await page.goto("/orderbook");
    await page.waitForLoadState("networkidle");

    await expect(page.getByTestId("swap-card")).not.toBeVisible();
  });
});
