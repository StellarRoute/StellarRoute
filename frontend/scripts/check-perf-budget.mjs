/**
 * check-perf-budget.mjs
 *
 * Main entry point for the performance budget checker.
 *
 * Usage:
 *   node scripts/check-perf-budget.mjs              # run checks
 *   node scripts/check-perf-budget.mjs --update-baseline  # capture new baseline
 *
 * Requirements: 2.3, 2.4, 3.3, 3.4, 4.3, 4.4, 5.3, 5.5, 6.1, 6.2, 6.5
 */

import fs from "fs";
import path from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

import {
  loadConfig,
  loadBaseline,
  buildBaseline,
  parseBundleSize,
  compareThreshold,
  computeOverallResult,
  generateContributorReport,
  parseStatsJson,
  writeContributorReport,
  buildResults,
} from "./perf-budget-utils.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_DIR = path.resolve(__dirname, "..");

const CONFIG_PATH = path.join(FRONTEND_DIR, "perf-budget.json");
const BASELINE_PATH = path.join(FRONTEND_DIR, "perf-baseline.json");
const RESULTS_PATH = path.join(FRONTEND_DIR, "perf-results.json");
const CONTRIBUTOR_REPORT_PATH = path.join(FRONTEND_DIR, "perf-contributor-report.txt");
const BUILD_DIR = path.join(FRONTEND_DIR, ".next");
const STATS_PATH = path.join(BUILD_DIR, "stats.json");
const MEASURE_TTI_SCRIPT = path.join(__dirname, "measure-tti.mjs");

// ---------------------------------------------------------------------------
// 9.1  runTTIMeasurement()
// ---------------------------------------------------------------------------

/**
 * Spawns measure-tti.mjs as a child process and returns the measured TTI in ms.
 * Exits with code 1 if the child process fails.
 *
 * @returns {number}  TTI in milliseconds
 */
function runTTIMeasurement() {
  let stdout;
  try {
    stdout = execSync(`node "${MEASURE_TTI_SCRIPT}"`, {
      encoding: "utf8",
      timeout: 30_000,
    });
  } catch (e) {
    const stderr = e.stderr ?? "";
    if (stderr.includes("production server") || stderr.includes("npm run start")) {
      console.error(
        "[perf-budget] ERROR: TTI measurement failed — production server is not running. " +
          "Start it with `npm run start` and try again."
      );
    } else {
      console.error(
        `[perf-budget] ERROR: TTI measurement failed: ${stderr || e.message}`
      );
    }
    process.exit(1);
  }

  let parsed;
  try {
    parsed = JSON.parse(stdout.trim());
  } catch {
    console.error(
      `[perf-budget] ERROR: Could not parse TTI measurement output: ${stdout}`
    );
    process.exit(1);
  }

  if (typeof parsed.ttiMs !== "number") {
    console.error(
      `[perf-budget] ERROR: TTI measurement returned unexpected value: ${JSON.stringify(parsed)}`
    );
    process.exit(1);
  }

  return parsed.ttiMs;
}

// ---------------------------------------------------------------------------
// 9.2  writeResults(results, outputPath)
// ---------------------------------------------------------------------------

/**
 * Serializes the PerfResults object to a JSON file.
 *
 * @param {object} results
 * @param {string} outputPath
 */
function writeResults(results, outputPath) {
  fs.writeFileSync(outputPath, JSON.stringify(results, null, 2) + "\n", "utf8");
}

// ---------------------------------------------------------------------------
// 9.3  printSummary(results, baseline?)
// ---------------------------------------------------------------------------

/**
 * Prints a human-readable summary to stdout.
 * Prints a single summary line when both checks pass.
 * Prints overage details and baseline deltas when checks fail.
 *
 * @param {object} results
 * @param {object|null} baseline
 */
function printSummary(results, baseline) {
  const bundleStatus = results.bundleSizePassed ? "✓" : "✗";
  const ttiStatus = results.ttiPassed ? "✓" : "✗";

  if (results.overallPassed) {
    console.log(
      `[perf-budget] ✓ All budgets met — ` +
        `Bundle: ${results.bundleSizeKb.toFixed(1)} KB / ${results.bundleSizeThresholdKb} KB, ` +
        `TTI: ${results.ttiMs.toFixed(0)} ms / ${results.ttiThresholdMs} ms`
    );
  } else {
    console.log("[perf-budget] Performance budget results:");
    console.log(
      `  ${bundleStatus} Bundle size: ${results.bundleSizeKb.toFixed(1)} KB ` +
        `(threshold: ${results.bundleSizeThresholdKb} KB)` +
        (results.bundleSizeOverageKb !== undefined
          ? `  ← OVER by ${results.bundleSizeOverageKb.toFixed(1)} KB`
          : "")
    );
    console.log(
      `  ${ttiStatus} TTI:         ${results.ttiMs.toFixed(0)} ms ` +
        `(threshold: ${results.ttiThresholdMs} ms)` +
        (results.ttiOverageMs !== undefined
          ? `  ← OVER by ${results.ttiOverageMs.toFixed(0)} ms`
          : "")
    );
  }

  // Baseline delta comparison
  if (baseline !== null && results.bundleSizeDeltaKb !== undefined) {
    const bundleDelta = results.bundleSizeDeltaKb;
    const ttiDelta = results.ttiDeltaMs;
    const bundleSign = bundleDelta >= 0 ? "+" : "";
    const ttiSign = ttiDelta >= 0 ? "+" : "";
    console.log(
      `[perf-budget] vs baseline — ` +
        `Bundle: ${bundleSign}${bundleDelta.toFixed(1)} KB, ` +
        `TTI: ${ttiSign}${ttiDelta.toFixed(0)} ms`
    );
  }
}

// ---------------------------------------------------------------------------
// 9.4  main(argv)
// ---------------------------------------------------------------------------

async function main(argv) {
  const updateBaseline = argv.includes("--update-baseline");

  // Load config (exits on error)
  const config = loadConfig(CONFIG_PATH);

  // Load baseline (warns and returns null if absent/malformed)
  const baseline = loadBaseline(BASELINE_PATH);

  // Measure bundle size
  console.log("[perf-budget] Measuring bundle size...");
  const bundleSizeKb = parseBundleSize(BUILD_DIR);

  // Measure TTI
  console.log("[perf-budget] Measuring TTI...");
  const ttiMs = runTTIMeasurement();

  // --update-baseline: capture new baseline and exit 0
  if (updateBaseline) {
    let commitSha = "unknown";
    try {
      commitSha = execSync("git rev-parse --short HEAD", { encoding: "utf8" }).trim();
    } catch {
      // Not in a git repo or git not available — use "unknown"
    }
    const newBaseline = buildBaseline(bundleSizeKb, ttiMs, commitSha);
    fs.writeFileSync(BASELINE_PATH, JSON.stringify(newBaseline, null, 2) + "\n", "utf8");
    console.log(
      `[perf-budget] Baseline updated — ` +
        `Bundle: ${bundleSizeKb.toFixed(1)} KB, TTI: ${ttiMs.toFixed(0)} ms ` +
        `(commit: ${commitSha})`
    );
    process.exit(0);
  }

  // Compare thresholds
  const bundleResult = compareThreshold(bundleSizeKb, config.bundleSizeKb);
  const ttiResult = compareThreshold(ttiMs, config.ttiMs);
  const { overallPassed } = computeOverallResult(bundleResult.passed, ttiResult.passed);

  // Build results object
  const results = buildResults({ bundleSizeKb, ttiMs }, config, baseline);

  // Write results file
  writeResults(results, RESULTS_PATH);

  // Generate contributor report if bundle size exceeded
  if (!bundleResult.passed) {
    console.log("[perf-budget] Bundle budget exceeded — generating contributor report...");
    try {
      // Re-run build with ANALYZE=true to get stats.json
      execSync("ANALYZE=true npm run build", {
        cwd: FRONTEND_DIR,
        stdio: "inherit",
        timeout: 120_000,
      });
      const modules = parseStatsJson(STATS_PATH);
      const entries = generateContributorReport(modules, bundleSizeKb);
      writeContributorReport(entries, CONTRIBUTOR_REPORT_PATH);
      console.log(`[perf-budget] Contributor report written to ${CONTRIBUTOR_REPORT_PATH}`);
      // Also print to stdout
      console.log("\n[perf-budget] Top bundle contributors:");
      for (const entry of entries) {
        console.log(
          `  ${entries.indexOf(entry) + 1}. ${entry.modulePath}  ${entry.sizeKb.toFixed(2)} KB  (${entry.percentageShare.toFixed(1)}%)`
        );
      }
    } catch (e) {
      console.warn(
        `[perf-budget] WARNING: Could not generate contributor report: ${e.message}`
      );
    }
  }

  // Print summary
  printSummary(results, baseline);

  process.exit(overallPassed ? 0 : 1);
}

main(process.argv.slice(2));
