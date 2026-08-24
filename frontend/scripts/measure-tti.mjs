/**
 * measure-tti.mjs
 *
 * Playwright TTI measurement helper.
 * Launched as a child process by check-perf-budget.mjs.
 *
 * Outputs a single JSON line to stdout: { "ttiMs": <number> }
 * Exits non-zero on page load timeout or Playwright not installed.
 *
 * Requirements: 3.1, 3.2, 3.6
 */

const PAGE_URL = process.env.PERF_URL ?? "http://localhost:3000/swap";
const TIMEOUT_MS = 10_000;

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error(
    "[perf-budget] ERROR: Playwright is not installed. " +
      "Run `npx playwright install chromium --with-deps` and try again."
  );
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
const page = await context.newPage();

// Inject Long Tasks PerformanceObserver before navigation so we capture
// any long tasks that occur during page load.
await page.addInitScript(() => {
  window.__longTasksEnd = 0;
  try {
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        window.__longTasksEnd = Math.max(
          window.__longTasksEnd,
          entry.startTime + entry.duration
        );
      }
    });
    observer.observe({ entryTypes: ["longtask"] });
  } catch {
    // Long Tasks API not available in this environment — fall back to 0
  }
});

try {
  await page.goto(PAGE_URL, { timeout: TIMEOUT_MS, waitUntil: "networkidle" });
} catch (e) {
  await browser.close();
  if (e.message && e.message.includes("Timeout")) {
    console.error(
      `[perf-budget] ERROR: Page load timeout after ${TIMEOUT_MS / 1000}s waiting for "${PAGE_URL}". ` +
        "Ensure the production server is running with `npm run start`."
    );
  } else {
    console.error(
      `[perf-budget] ERROR: Failed to load "${PAGE_URL}": ${e.message}. ` +
        "Ensure the production server is running with `npm run start`."
    );
  }
  process.exit(1);
}

const ttiMs = await page.evaluate(() => {
  // domInteractive from PerformanceTiming (ms since navigation start)
  const domInteractive =
    performance.timing.domInteractive - performance.timing.navigationStart;
  // Last long task end time (ms since navigation start, 0 if none)
  const lastLongTask = window.__longTasksEnd ?? 0;
  return Math.max(domInteractive, lastLongTask);
});

await browser.close();

// Output a single JSON line for the parent process to parse
process.stdout.write(JSON.stringify({ ttiMs }) + "\n");
