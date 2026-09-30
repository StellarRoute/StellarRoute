/**
 * E2E spec: /analytics page
 *
 * Issue #1227 — Playwright coverage for /analytics
 *
 * Scope:
 *   - Visit /analytics with the analytics flag enabled (injected via initScript)
 *   - Mock /metrics/cache and /metrics/pool endpoints — no live API required
 *   - Assert the "Analytics" heading renders
 *   - Assert mocked cache metrics (hit ratio, hits, misses) render
 *   - Assert mocked pool stats (primary utilisation) render
 *   - Assert the disabled-flag fallback renders when the flag is off
 *
 * Additive constraint: no /swap files are changed; no production env vars are
 * set; NEXT_PUBLIC_FEATURE_ANALYTICS is exercised only inside initScript so it
 * never reaches Vercel or .env.example.
 *
 * Production files touched by this PR: none (new file only).
 */

import { test, expect } from "@playwright/test";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function cacheMetricsFixture() {
  return {
    quote_hits: 4200,
    quote_misses: 800,
    hit_ratio: 0.84,
    stale_quote_rejections: 12,
    stale_inputs_excluded: 3,
  };
}

function poolStatsFixture() {
  return {
    primary: {
      in_use: 4,
      idle: 6,
      max_connections: 10,
      utilisation: 0.4,
    },
    replica: null,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Route both metrics endpoints with mocked JSON responses. */
async function mockMetricsEndpoints(page: import("@playwright/test").Page) {
  await page.route("**/metrics/cache", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(cacheMetricsFixture()),
    });
  });

  await page.route("**/metrics/pool", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(poolStatsFixture()),
    });
  });
}

/** Enable the analytics feature flag via initScript before page load. */
async function enableAnalyticsFlag(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    // The feature flag is read from the NEXT_PUBLIC_FEATURE_ANALYTICS env var
    // during SSR/build. In e2e we override it on the window-level flags object
    // that useFeatureFlag reads in the browser.
    (
      window as unknown as {
        __STELLAR_ROUTE_FLAGS__?: Record<string, boolean>;
      }
    ).__STELLAR_ROUTE_FLAGS__ = { analytics: true };
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

test.describe("/analytics page", () => {
  /**
   * 1. Heading renders — basic smoke test that the route is reachable and the
   *    h1 "Analytics" is present regardless of flag state.
   */
  test("renders the Analytics heading", async ({ page }) => {
    await mockMetricsEndpoints(page);
    await enableAnalyticsFlag(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    await expect(
      page.getByRole("heading", { name: /analytics/i, level: 1 })
    ).toBeVisible();
  });

  /**
   * 2. With flag enabled and mocked endpoints, cache metrics card renders
   *    the hit ratio and raw hit/miss counters from the fixture.
   */
  test("renders mocked cache metrics when flag is enabled", async ({ page }) => {
    await mockMetricsEndpoints(page);
    await enableAnalyticsFlag(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    // AnalyticsDashboard shows formatPercent(hit_ratio) = "84.0%"
    await expect(page.getByText("84.0%")).toBeVisible({ timeout: 5000 });

    // Hit and miss counts from the fixture
    await expect(page.getByText("4,200")).toBeVisible();
    await expect(page.getByText("800")).toBeVisible();
  });

  /**
   * 3. Pool stats card renders with primary utilisation from the fixture.
   */
  test("renders mocked pool stats when flag is enabled", async ({ page }) => {
    await mockMetricsEndpoints(page);
    await enableAnalyticsFlag(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    // PoolStatsCard heading
    await expect(
      page.getByRole("heading", { name: /primary pool/i })
    ).toBeVisible({ timeout: 5000 });

    // formatPercent(0.4) = "40.0%"
    await expect(page.getByText("40.0%")).toBeVisible();
  });

  /**
   * 4. Progressbar for primary pool is present and has the right aria attributes.
   */
  test("primary pool progressbar has correct aria-valuenow", async ({ page }) => {
    await mockMetricsEndpoints(page);
    await enableAnalyticsFlag(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    const progressbar = page.getByRole("progressbar", {
      name: /primary pool utilisation/i,
    });
    await expect(progressbar).toBeVisible({ timeout: 5000 });
    // Math.round(0.4 * 100) = 40
    await expect(progressbar).toHaveAttribute("aria-valuenow", "40");
  });

  /**
   * 5. Refresh button is present and clickable; re-fetches both endpoints.
   */
  test("Refresh button triggers re-fetch of metrics endpoints", async ({ page }) => {
    let cacheHits = 0;
    let poolHits = 0;

    await page.route("**/metrics/cache", async (route) => {
      cacheHits++;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(cacheMetricsFixture()),
      });
    });

    await page.route("**/metrics/pool", async (route) => {
      poolHits++;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(poolStatsFixture()),
      });
    });

    await enableAnalyticsFlag(page);
    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    const refreshBtn = page.getByRole("button", {
      name: /refresh analytics metrics/i,
    });
    await expect(refreshBtn).toBeVisible();

    // Record baseline call counts after initial load
    const cacheAfterLoad = cacheHits;
    const poolAfterLoad = poolHits;

    await refreshBtn.click();
    await page.waitForTimeout(500);

    expect(cacheHits).toBeGreaterThan(cacheAfterLoad);
    expect(poolHits).toBeGreaterThan(poolAfterLoad);
  });

  /**
   * 6. When the analytics flag is unset (default / production state), the page
   *    shows the disabled-flag fallback message rather than live metrics.
   *    This confirms no user-visible difference on production deployments.
   */
  test("shows disabled-flag fallback when NEXT_PUBLIC_FEATURE_ANALYTICS is unset", async ({
    page,
  }) => {
    // Do NOT call enableAnalyticsFlag — simulate production default (flag off)
    await mockMetricsEndpoints(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    // The AnalyticsPageClient disabled branch renders this copy
    await expect(
      page.getByText(/analytics preview disabled/i)
    ).toBeVisible({ timeout: 5000 });

    // The live metrics card must NOT be present
    await expect(
      page.getByRole("heading", { name: /primary pool/i })
    ).not.toBeVisible();
  });

  /**
   * 7. The spec never navigates to /swap. Confirm no swap-card element appears.
   */
  test("does not mount the swap card", async ({ page }) => {
    await mockMetricsEndpoints(page);
    await enableAnalyticsFlag(page);

    await page.goto("/analytics");
    await page.waitForLoadState("networkidle");

    await expect(page.getByTestId("swap-card")).not.toBeVisible();
  });
});
