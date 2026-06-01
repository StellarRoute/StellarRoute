/**
 * perf-budget-utils.mjs
 *
 * Pure utility functions for the performance budget checker.
 * Node.js ESM module — no external dependencies beyond Node.js built-ins.
 */

import fs from "fs";

// ---------------------------------------------------------------------------
// 3.1  loadConfig(configPath)
// ---------------------------------------------------------------------------

/**
 * Reads and validates the performance budget config file.
 *
 * Validates:
 *   - `version`      — must be a non-empty string
 *   - `bundleSizeKb` — must be a positive number
 *   - `ttiMs`        — must be a positive number
 *
 * Calls process.exit(1) with a descriptive message on any error.
 *
 * @param {string} configPath  Absolute or relative path to perf-budget.json
 * @returns {object}           Validated config object
 */
export function loadConfig(configPath) {
  let raw;

  try {
    raw = JSON.parse(fs.readFileSync(configPath, "utf8"));
  } catch (e) {
    if (e.code === "ENOENT") {
      console.error(
        `[perf-budget] ERROR: Config file not found at "${configPath}"`
      );
    } else if (e instanceof SyntaxError) {
      console.error(
        `[perf-budget] ERROR: Config file at "${configPath}" contains invalid JSON: ${e.message}`
      );
    } else {
      console.error(
        `[perf-budget] ERROR: Cannot read config at "${configPath}": ${e.message}`
      );
    }
    process.exit(1);
  }

  // Presence checks
  for (const field of ["version", "bundleSizeKb", "ttiMs"]) {
    if (raw[field] === undefined || raw[field] === null) {
      console.error(
        `[perf-budget] ERROR: Missing required field "${field}" in "${configPath}"`
      );
      process.exit(1);
    }
  }

  // Type check: version must be a string
  if (typeof raw.version !== "string") {
    console.error(
      `[perf-budget] ERROR: Field "version" must be a string, got ${typeof raw.version}`
    );
    process.exit(1);
  }

  // Type + range check: bundleSizeKb must be a positive number
  if (typeof raw.bundleSizeKb !== "number" || raw.bundleSizeKb <= 0) {
    console.error(
      `[perf-budget] ERROR: Field "bundleSizeKb" must be a positive number (got ${raw.bundleSizeKb})`
    );
    process.exit(1);
  }

  // Type + range check: ttiMs must be a positive number
  if (typeof raw.ttiMs !== "number" || raw.ttiMs <= 0) {
    console.error(
      `[perf-budget] ERROR: Field "ttiMs" must be a positive number (got ${raw.ttiMs})`
    );
    process.exit(1);
  }

  return raw;
}

// ---------------------------------------------------------------------------
// 3.2  loadBaseline(baselinePath)
// ---------------------------------------------------------------------------

/**
 * Reads the baseline snapshot file.
 *
 * If the file is absent or contains invalid JSON, emits a warning to stderr
 * and returns null — never calls process.exit.
 *
 * @param {string} baselinePath  Path to perf-baseline.json
 * @returns {object|null}        Baseline object or null
 */
export function loadBaseline(baselinePath) {
  try {
    const raw = JSON.parse(fs.readFileSync(baselinePath, "utf8"));
    return raw;
  } catch (e) {
    if (e.code === "ENOENT") {
      console.warn(
        `[perf-budget] WARNING: Baseline file not found at "${baselinePath}". Continuing without baseline comparison.`
      );
    } else {
      console.warn(
        `[perf-budget] WARNING: Could not parse baseline file at "${baselinePath}": ${e.message}. Continuing without baseline comparison.`
      );
    }
    return null;
  }
}

// ---------------------------------------------------------------------------
// 3.3  compareThreshold(measured, threshold)
// ---------------------------------------------------------------------------

/**
 * Compares a measured value against a threshold.
 *
 * @param {number} measured   The measured value
 * @param {number} threshold  The budget threshold
 * @returns {{ passed: true } | { passed: false, overage: number }}
 */
export function compareThreshold(measured, threshold) {
  if (measured <= threshold) {
    return { passed: true };
  }
  return { passed: false, overage: measured - threshold };
}

// ---------------------------------------------------------------------------
// 3.4  computeOverallResult(bundlePassed, ttiPassed)
// ---------------------------------------------------------------------------

/**
 * Computes the overall pass/fail result from the two individual checks.
 *
 * @param {boolean} bundlePassed
 * @param {boolean} ttiPassed
 * @returns {{ overallPassed: boolean }}
 */
export function computeOverallResult(bundlePassed, ttiPassed) {
  return { overallPassed: bundlePassed && ttiPassed };
}

// ---------------------------------------------------------------------------
// 3.5  computeDeltas(current, baseline)
// ---------------------------------------------------------------------------

/**
 * Computes the delta between current measurements and a baseline.
 * Negative values indicate improvement.
 *
 * @param {{ bundleSizeKb: number, ttiMs: number }} current
 * @param {{ bundleSizeKb: number, ttiMs: number }} baseline
 * @returns {{ bundleSizeDeltaKb: number, ttiDeltaMs: number }}
 */
export function computeDeltas(current, baseline) {
  return {
    bundleSizeDeltaKb: current.bundleSizeKb - baseline.bundleSizeKb,
    ttiDeltaMs: current.ttiMs - baseline.ttiMs,
  };
}

// ---------------------------------------------------------------------------
// 3.6  buildResults(measured, config, baseline?)
// ---------------------------------------------------------------------------

/**
 * Assembles the full PerfResults object.
 *
 * @param {{ bundleSizeKb: number, ttiMs: number }} measured
 * @param {{ bundleSizeKb: number, ttiMs: number }} config   Threshold values
 * @param {{ bundleSizeKb: number, ttiMs: number }|null} [baseline]
 * @returns {object}  PerfResults
 */
export function buildResults(measured, config, baseline = null) {
  const bundleResult = compareThreshold(measured.bundleSizeKb, config.bundleSizeKb);
  const ttiResult = compareThreshold(measured.ttiMs, config.ttiMs);
  const { overallPassed } = computeOverallResult(bundleResult.passed, ttiResult.passed);

  /** @type {object} */
  const results = {
    bundleSizeKb: measured.bundleSizeKb,
    ttiMs: measured.ttiMs,
    bundleSizeThresholdKb: config.bundleSizeKb,
    ttiThresholdMs: config.ttiMs,
    bundleSizePassed: bundleResult.passed,
    ttiPassed: ttiResult.passed,
    overallPassed,
    timestamp: new Date().toISOString(),
  };

  // Optional overage fields — only present when the check failed
  if (!bundleResult.passed) {
    results.bundleSizeOverageKb = bundleResult.overage;
  }
  if (!ttiResult.passed) {
    results.ttiOverageMs = ttiResult.overage;
  }

  // Optional baseline delta fields — only present when a baseline is provided
  if (baseline !== null) {
    const deltas = computeDeltas(measured, baseline);
    results.baselineBundleSizeKb = baseline.bundleSizeKb;
    results.baselineTtiMs = baseline.ttiMs;
    results.bundleSizeDeltaKb = deltas.bundleSizeDeltaKb;
    results.ttiDeltaMs = deltas.ttiDeltaMs;
  }

  return results;
}

// ---------------------------------------------------------------------------
// 3.7  buildBaseline(bundleSizeKb, ttiMs, commitSha)
// ---------------------------------------------------------------------------

/**
 * Assembles a BaselineReport object from the current measurements.
 *
 * @param {number} bundleSizeKb
 * @param {number} ttiMs
 * @param {string} commitSha
 * @returns {{ bundleSizeKb: number, ttiMs: number, commitSha: string, capturedAt: string }}
 */
export function buildBaseline(bundleSizeKb, ttiMs, commitSha) {
  return {
    bundleSizeKb,
    ttiMs,
    commitSha,
    capturedAt: new Date().toISOString(),
  };
}

// ---------------------------------------------------------------------------
// 3.8  generateContributorReport(modules, totalKb)
// ---------------------------------------------------------------------------

/**
 * Generates the top-10 module contributor report.
 *
 * @param {Array<{ name: string, gzipSizeKb: number }>} modules
 * @param {number} totalKb  Total bundle size in KB (used for percentage calculation)
 * @returns {Array<{ modulePath: string, sizeKb: number, percentageShare: number }>}
 */
export function generateContributorReport(modules, totalKb) {
  return modules
    .slice() // avoid mutating the input array
    .sort((a, b) => b.gzipSizeKb - a.gzipSizeKb)
    .slice(0, 10)
    .map((m) => ({
      modulePath: m.name,
      sizeKb: m.gzipSizeKb,
      percentageShare: (m.gzipSizeKb / totalKb) * 100,
    }));
}

// ---------------------------------------------------------------------------
// Helper: computeBundleSize(chunkSizesKb)
// ---------------------------------------------------------------------------

/**
 * Sums an array of chunk sizes (in KB).
 * Exported so property tests can verify the summation logic directly.
 *
 * @param {number[]} chunkSizesKb
 * @returns {number}  Total size in KB
 */
export function computeBundleSize(chunkSizesKb) {
  return chunkSizesKb.reduce((total, size) => total + size, 0);
}

// ---------------------------------------------------------------------------
// 6.1  parseBundleSize(buildDir)
// ---------------------------------------------------------------------------

import zlib from "zlib";
import path from "path";

/**
 * Reads .next/build-manifest.json and computes the total gzip-compressed size
 * of all JavaScript chunks attributed to the /swap route (including shared
 * /_app chunks).
 *
 * Exits with code 1 and a descriptive message if:
 *   - The .next/ directory is missing
 *   - build-manifest.json is absent
 *   - The /swap route is not present in the manifest
 *
 * @param {string} buildDir  Path to the .next/ directory
 * @returns {number}         Total gzip size in KB
 */
export function parseBundleSize(buildDir) {
  const manifestPath = path.join(buildDir, "build-manifest.json");

  // Check .next/ exists
  if (!fs.existsSync(buildDir)) {
    console.error(
      `[perf-budget] ERROR: Build directory not found at "${buildDir}". Run \`npm run build\` first.`
    );
    process.exit(1);
  }

  // Check build-manifest.json exists
  if (!fs.existsSync(manifestPath)) {
    console.error(
      `[perf-budget] ERROR: build-manifest.json not found at "${manifestPath}". Run \`npm run build\` first.`
    );
    process.exit(1);
  }

  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  } catch (e) {
    console.error(
      `[perf-budget] ERROR: Failed to parse build-manifest.json: ${e.message}`
    );
    process.exit(1);
  }

  const pages = manifest.pages ?? {};

  // Check /swap route exists in manifest
  if (!pages["/swap"]) {
    const available = Object.keys(pages).join(", ");
    console.error(
      `[perf-budget] ERROR: Route "/swap" not found in build manifest. Available routes: ${available}`
    );
    process.exit(1);
  }

  // Collect all unique JS chunks for /swap + /_app (shared)
  const swapChunks = new Set([
    ...(pages["/swap"] ?? []),
    ...(pages["/_app"] ?? []),
  ]);

  let totalBytes = 0;
  for (const chunk of swapChunks) {
    if (!chunk.endsWith(".js")) continue;
    // Chunks are relative paths like "_next/static/chunks/foo.js"
    // Strip the leading "_next/" since buildDir is already .next/
    const relativePath = chunk.startsWith("_next/")
      ? chunk.slice("_next/".length)
      : chunk;
    const filePath = path.join(buildDir, relativePath);

    if (!fs.existsSync(filePath)) {
      // Skip missing chunks (e.g. already-hashed files that moved)
      continue;
    }

    const content = fs.readFileSync(filePath);
    totalBytes += zlib.gzipSync(content).length;
  }

  return totalBytes / 1024; // KB
}

// ---------------------------------------------------------------------------
// 7.1  parseStatsJson(statsPath)
// ---------------------------------------------------------------------------

/**
 * Reads the .next/stats.json file produced by @next/bundle-analyzer
 * (when ANALYZE=true is set during next build) and returns a flat list
 * of modules with their name and gzip size in KB.
 *
 * @param {string} statsPath  Path to .next/stats.json
 * @returns {Array<{ name: string, gzipSizeKb: number }>}
 */
export function parseStatsJson(statsPath) {
  if (!fs.existsSync(statsPath)) {
    console.error(
      `[perf-budget] ERROR: stats.json not found at "${statsPath}". ` +
        `Run \`ANALYZE=true npm run build\` to generate it.`
    );
    process.exit(1);
  }

  let stats;
  try {
    stats = JSON.parse(fs.readFileSync(statsPath, "utf8"));
  } catch (e) {
    console.error(
      `[perf-budget] ERROR: Failed to parse stats.json: ${e.message}`
    );
    process.exit(1);
  }

  // webpack stats JSON has a top-level `modules` array; each module has
  // `name` and `gzipSize` (in bytes).
  const modules = stats.modules ?? [];
  return modules
    .filter((m) => typeof m.name === "string" && typeof m.gzipSize === "number")
    .map((m) => ({
      name: m.name,
      gzipSizeKb: m.gzipSize / 1024,
    }));
}

// ---------------------------------------------------------------------------
// 7.2  writeContributorReport(entries, outputPath)
// ---------------------------------------------------------------------------

/**
 * Formats and writes the contributor report to a text file.
 *
 * Each line format: `<rank>. <modulePath>  <sizeKb> KB  (<percentageShare>%)`
 *
 * @param {Array<{ modulePath: string, sizeKb: number, percentageShare: number }>} entries
 * @param {string} outputPath  Path to write the report (e.g. perf-contributor-report.txt)
 */
export function writeContributorReport(entries, outputPath) {
  const lines = entries.map(
    (entry, i) =>
      `${i + 1}. ${entry.modulePath}  ${entry.sizeKb.toFixed(2)} KB  (${entry.percentageShare.toFixed(1)}%)`
  );
  const content = lines.join("\n") + "\n";
  fs.writeFileSync(outputPath, content, "utf8");
}
