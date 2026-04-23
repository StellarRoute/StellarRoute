import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright configuration for mobile visual regression tests.
 * Feature: mobile-swap-experience
 *
 * Screenshot diff threshold: 0.2 (20% pixel difference tolerance).
 * CI will fail when a diff exceeds this threshold (toHaveScreenshot default behaviour).
 */
export default defineConfig({
  testDir: "./e2e",
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 1 : 0,
  /* Reporter */
  reporter: process.env.CI ? "github" : "list",
  use: {
    /* Base URL for the Next.js dev server */
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:3000",
    /* Collect trace on first retry */
    trace: "on-first-retry",
    /* Screenshot comparison threshold — CI fails when diff exceeds this */
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.02, // 2% pixel ratio threshold
    },
  },
  projects: [
    {
      name: "chromium-mobile",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "edge-cases",
      use: {
        ...devices["Desktop Chrome"],
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
      },
      testMatch: "**/quote-edge-cases.spec.ts",
    },
  ],
  /* Start the Next.js dev server before running tests */
  webServer: {
    command: "npm run dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
