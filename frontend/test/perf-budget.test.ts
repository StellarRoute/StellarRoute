/**
 * Tests for perf-budget-utils.mjs
 *
 * Covers:
 *  - Property-based tests (fast-check) for Properties 1, 3, 4, 5, 6, 8, 9, 10
 *  - Example-based unit tests for loadConfig, generateContributorReport, loadBaseline
 */

import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
  type MockInstance,
} from "vitest";
import * as fc from "fast-check";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

import {
  loadConfig,
  loadBaseline,
  compareThreshold,
  computeOverallResult,
  computeDeltas,
  buildResults,
  buildBaseline,
  generateContributorReport,
  computeBundleSize,
} from "../scripts/perf-budget-utils.mjs";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Write an object as JSON to a temp file and return the path. */
function writeTempJson(obj: unknown): string {
  const tmpFile = path.join(os.tmpdir(), `perf-budget-test-${Date.now()}-${Math.random().toString(36).slice(2)}.json`);
  fs.writeFileSync(tmpFile, JSON.stringify(obj), "utf8");
  return tmpFile;
}

/** Write raw text to a temp file and return the path. */
function writeTempText(text: string): string {
  const tmpFile = path.join(os.tmpdir(), `perf-budget-test-${Date.now()}-${Math.random().toString(36).slice(2)}.json`);
  fs.writeFileSync(tmpFile, text, "utf8");
  return tmpFile;
}

// ---------------------------------------------------------------------------
// Property 1: Malformed config always produces non-zero exit and descriptive error
// Feature: performance-budget-checks, Property 1: Malformed config always produces non-zero exit
// ---------------------------------------------------------------------------

describe("loadConfig validation", () => {
  let exitSpy: MockInstance;

  beforeEach(() => {
    exitSpy = vi.spyOn(process, "exit").mockImplementation((code?: number | string | null | undefined) => {
      throw new Error(`process.exit(${code})`);
    });
  });

  afterEach(() => {
    exitSpy.mockRestore();
  });

  // Feature: performance-budget-checks, Property 1: Malformed config always produces non-zero exit
  it("rejects any config missing a required field (property test)", () => {
    // Validates: Requirements 1.4
    fc.assert(
      fc.property(
        fc.constantFrom("version", "bundleSizeKb", "ttiMs"),
        (missingField) => {
          const config: Record<string, unknown> = {
            version: "1",
            bundleSizeKb: 250,
            ttiMs: 3000,
          };
          delete config[missingField];
          const tmpFile = writeTempJson(config);
          try {
            expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
          } finally {
            fs.unlinkSync(tmpFile);
          }
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 3 & 4: Threshold comparison correctness
// Feature: performance-budget-checks, Property 3 & 4: Threshold comparison is correct
// ---------------------------------------------------------------------------

describe("compareThreshold", () => {
  // Feature: performance-budget-checks, Property 3: Threshold comparison is correct when measured > threshold
  it("returns passed:false with correct overage when measured > threshold (property test)", () => {
    // Validates: Requirements 2.3, 3.3
    fc.assert(
      fc.property(
        fc.float({ min: Math.fround(0.1), max: Math.fround(1000), noNaN: true }),
        fc.float({ min: Math.fround(0.1), max: Math.fround(1000), noNaN: true }),
        (a, b) => {
          const measured = Math.max(a, b);
          const threshold = Math.min(a, b);
          fc.pre(measured > threshold);

          const result = compareThreshold(measured, threshold);
          expect(result.passed).toBe(false);
          // overage must be present and approximately equal to measured - threshold
          expect((result as { passed: false; overage: number }).overage).toBeCloseTo(
            measured - threshold,
            4
          );
        }
      ),
      { numRuns: 20 }
    );
  });

  // Feature: performance-budget-checks, Property 4: Threshold comparison is correct when measured <= threshold
  it("returns passed:true when measured <= threshold (property test)", () => {
    // Validates: Requirements 2.4, 3.4
    fc.assert(
      fc.property(
        fc.float({ min: Math.fround(0.1), max: Math.fround(1000), noNaN: true }),
        fc.float({ min: Math.fround(0.1), max: Math.fround(1000), noNaN: true }),
        (a, b) => {
          const measured = Math.min(a, b);
          const threshold = Math.max(a, b);
          // measured <= threshold is guaranteed by the min/max above

          const result = compareThreshold(measured, threshold);
          expect(result.passed).toBe(true);
          expect((result as { passed: true; overage?: number }).overage).toBeUndefined();
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 5: Results file faithfully records both measured values
// Feature: performance-budget-checks, Property 5: Results file faithfully records both measured values
// ---------------------------------------------------------------------------

describe("buildResults round-trip", () => {
  // Feature: performance-budget-checks, Property 5: Results file faithfully records both measured values
  it("serialised results preserve bundleSizeKb and ttiMs (property test)", () => {
    // Validates: Requirements 2.5, 3.5
    fc.assert(
      fc.property(
        fc.float({ min: Math.fround(0.1), max: Math.fround(2000), noNaN: true }),
        fc.float({ min: Math.fround(1), max: Math.fround(30000), noNaN: true }),
        (bundleSizeKb, ttiMs) => {
          const config = { bundleSizeKb: 250, ttiMs: 3000 };
          const results = buildResults(
            { bundleSizeKb, ttiMs },
            config,
            null
          );
          const parsed = JSON.parse(JSON.stringify(results));
          expect(parsed.bundleSizeKb).toBeCloseTo(bundleSizeKb, 5);
          expect(parsed.ttiMs).toBeCloseTo(ttiMs, 5);
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 6: Contributor report contains the top-N largest modules
// Feature: performance-budget-checks, Property 6: Contributor report contains the top-N largest modules
// ---------------------------------------------------------------------------

describe("generateContributorReport", () => {
  // Feature: performance-budget-checks, Property 6: Contributor report contains the top-N largest modules
  it("returns at most 10 entries, sorted descending, with correct percentages (property test)", () => {
    // Validates: Requirements 4.1, 4.2
    fc.assert(
      fc.property(
        fc.array(
          fc.record({
            name: fc.string({ minLength: 1, maxLength: 80 }),
            gzipSizeKb: fc.float({ min: Math.fround(0.1), max: Math.fround(500), noNaN: true }),
          }),
          { minLength: 1, maxLength: 50 }
        ),
        (modules) => {
          const totalKb = modules.reduce((s, m) => s + m.gzipSizeKb, 0);
          const report = generateContributorReport(modules, totalKb);

          // At most 10 entries
          expect(report.length).toBeLessThanOrEqual(10);

          // Sorted descending by sizeKb
          for (let i = 1; i < report.length; i++) {
            expect(report[i - 1].sizeKb).toBeGreaterThanOrEqual(report[i].sizeKb);
          }

          // Percentages are correct for each entry
          for (const entry of report) {
            const expected = (entry.sizeKb / totalKb) * 100;
            expect(entry.percentageShare).toBeCloseTo(expected, 3);
          }
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 8: Baseline capture records all required fields
// Feature: performance-budget-checks, Property 8: Baseline capture records all required fields
// ---------------------------------------------------------------------------

describe("buildBaseline", () => {
  // Feature: performance-budget-checks, Property 8: Baseline capture records all required fields
  it("output has all 4 required fields with correct values (property test)", () => {
    // Validates: Requirements 5.2, 5.3
    fc.assert(
      fc.property(
        fc.float({ min: Math.fround(0.1), max: Math.fround(2000), noNaN: true }),
        fc.float({ min: Math.fround(1), max: Math.fround(30000), noNaN: true }),
        fc.hexaString({ minLength: 7, maxLength: 40 }),
        (bundleSizeKb, ttiMs, commitSha) => {
          const baseline = buildBaseline(bundleSizeKb, ttiMs, commitSha);

          expect(baseline.bundleSizeKb).toBeCloseTo(bundleSizeKb, 5);
          expect(baseline.ttiMs).toBeCloseTo(ttiMs, 5);
          expect(baseline.commitSha).toBe(commitSha);
          // capturedAt must be a valid ISO 8601 string
          expect(typeof baseline.capturedAt).toBe("string");
          expect(new Date(baseline.capturedAt).toISOString()).toBe(baseline.capturedAt);
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 9: Delta comparison equals current minus baseline
// Feature: performance-budget-checks, Property 9: Delta comparison equals current minus baseline
// ---------------------------------------------------------------------------

describe("computeDeltas", () => {
  // Feature: performance-budget-checks, Property 9: Delta comparison equals current minus baseline
  it("deltas equal current minus baseline (property test)", () => {
    // Validates: Requirements 5.4
    fc.assert(
      fc.property(
        fc.float({ min: Math.fround(0.1), max: Math.fround(2000), noNaN: true }),
        fc.float({ min: Math.fround(1), max: Math.fround(30000), noNaN: true }),
        fc.float({ min: Math.fround(0.1), max: Math.fround(2000), noNaN: true }),
        fc.float({ min: Math.fround(1), max: Math.fround(30000), noNaN: true }),
        (currentBundle, currentTTI, baselineBundle, baselineTTI) => {
          const deltas = computeDeltas(
            { bundleSizeKb: currentBundle, ttiMs: currentTTI },
            { bundleSizeKb: baselineBundle, ttiMs: baselineTTI }
          );
          expect(deltas.bundleSizeDeltaKb).toBeCloseTo(
            currentBundle - baselineBundle,
            5
          );
          expect(deltas.ttiDeltaMs).toBeCloseTo(currentTTI - baselineTTI, 5);
        }
      ),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// Property 10: Overall exit code is zero iff both individual checks pass
// Feature: performance-budget-checks, Property 10: Overall exit code is zero iff both individual checks pass
// ---------------------------------------------------------------------------

describe("computeOverallResult", () => {
  // Feature: performance-budget-checks, Property 10: Overall exit code is zero iff both individual checks pass
  it("overallPassed is true iff both inputs are true (property test)", () => {
    // Validates: Requirements 6.2
    fc.assert(
      fc.property(fc.boolean(), fc.boolean(), (bundlePassed, ttiPassed) => {
        const result = computeOverallResult(bundlePassed, ttiPassed);
        expect(result.overallPassed).toBe(bundlePassed && ttiPassed);
      }),
      { numRuns: 20 }
    );
  });
});

// ---------------------------------------------------------------------------
// 4.10 Example-based unit tests for loadConfig
// ---------------------------------------------------------------------------

describe("loadConfig — example-based unit tests", () => {
  let exitSpy: MockInstance;

  beforeEach(() => {
    exitSpy = vi.spyOn(process, "exit").mockImplementation((code?: number | string | null | undefined) => {
      throw new Error(`process.exit(${code})`);
    });
  });

  afterEach(() => {
    exitSpy.mockRestore();
  });

  it("returns the config when all fields are valid", () => {
    const cfg = { version: "1", bundleSizeKb: 250, ttiMs: 3000 };
    const tmpFile = writeTempJson(cfg);
    try {
      const result = loadConfig(tmpFile);
      expect(result.version).toBe("1");
      expect(result.bundleSizeKb).toBe(250);
      expect(result.ttiMs).toBe(3000);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'version' field is missing", () => {
    const tmpFile = writeTempJson({ bundleSizeKb: 250, ttiMs: 3000 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'bundleSizeKb' field is missing", () => {
    const tmpFile = writeTempJson({ version: "1", ttiMs: 3000 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'ttiMs' field is missing", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: 250 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'bundleSizeKb' is zero", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: 0, ttiMs: 3000 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'ttiMs' is zero", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: 250, ttiMs: 0 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'bundleSizeKb' is negative", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: -10, ttiMs: 3000 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'ttiMs' is negative", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: 250, ttiMs: -1 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'bundleSizeKb' is a non-number string", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: "250", ttiMs: 3000 });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when 'ttiMs' is a non-number type (boolean)", () => {
    const tmpFile = writeTempJson({ version: "1", bundleSizeKb: 250, ttiMs: true });
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("exits 1 when the file does not exist", () => {
    expect(() => loadConfig("/nonexistent/path/perf-budget.json")).toThrow(
      /process\.exit\(1\)/
    );
  });

  it("exits 1 when the file contains invalid JSON", () => {
    const tmpFile = writeTempText("{ not valid json }");
    try {
      expect(() => loadConfig(tmpFile)).toThrow(/process\.exit\(1\)/);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });
});

// ---------------------------------------------------------------------------
// 4.11 Example-based unit tests for generateContributorReport
// ---------------------------------------------------------------------------

describe("generateContributorReport — example-based unit tests", () => {
  it("returns all modules when fewer than 10 are provided", () => {
    const modules = [
      { name: "a.js", gzipSizeKb: 10 },
      { name: "b.js", gzipSizeKb: 5 },
      { name: "c.js", gzipSizeKb: 3 },
    ];
    const totalKb = 18;
    const report = generateContributorReport(modules, totalKb);
    expect(report.length).toBe(3);
  });

  it("returns exactly 10 entries when exactly 10 modules are provided", () => {
    const modules = Array.from({ length: 10 }, (_, i) => ({
      name: `mod-${i}.js`,
      gzipSizeKb: (i + 1) * 5,
    }));
    const totalKb = modules.reduce((s, m) => s + m.gzipSizeKb, 0);
    const report = generateContributorReport(modules, totalKb);
    expect(report.length).toBe(10);
  });

  it("returns only the top 10 when more than 10 modules are provided", () => {
    const modules = Array.from({ length: 25 }, (_, i) => ({
      name: `mod-${i}.js`,
      gzipSizeKb: (i + 1) * 2,
    }));
    const totalKb = modules.reduce((s, m) => s + m.gzipSizeKb, 0);
    const report = generateContributorReport(modules, totalKb);
    expect(report.length).toBe(10);
    // The top entry should be the largest module (mod-24.js = 50 KB)
    expect(report[0].sizeKb).toBe(50);
    expect(report[0].modulePath).toBe("mod-24.js");
  });

  it("entries are sorted descending by sizeKb", () => {
    const modules = [
      { name: "small.js", gzipSizeKb: 1 },
      { name: "large.js", gzipSizeKb: 100 },
      { name: "medium.js", gzipSizeKb: 50 },
    ];
    const totalKb = 151;
    const report = generateContributorReport(modules, totalKb);
    expect(report[0].modulePath).toBe("large.js");
    expect(report[1].modulePath).toBe("medium.js");
    expect(report[2].modulePath).toBe("small.js");
  });

  it("percentageShare values are computed correctly", () => {
    const modules = [
      { name: "a.js", gzipSizeKb: 50 },
      { name: "b.js", gzipSizeKb: 50 },
    ];
    const totalKb = 100;
    const report = generateContributorReport(modules, totalKb);
    for (const entry of report) {
      expect(entry.percentageShare).toBeCloseTo(50, 5);
    }
  });

  it("each entry has modulePath, sizeKb, and percentageShare fields", () => {
    const modules = [{ name: "x.js", gzipSizeKb: 20 }];
    const report = generateContributorReport(modules, 20);
    expect(report[0]).toHaveProperty("modulePath");
    expect(report[0]).toHaveProperty("sizeKb");
    expect(report[0]).toHaveProperty("percentageShare");
  });
});

// ---------------------------------------------------------------------------
// 4.12 Example-based unit tests for loadBaseline
// ---------------------------------------------------------------------------

describe("loadBaseline — example-based unit tests", () => {
  let warnSpy: MockInstance;

  beforeEach(() => {
    warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
  });

  afterEach(() => {
    warnSpy.mockRestore();
  });

  it("returns null and emits a warning when the file is absent", () => {
    const result = loadBaseline("/nonexistent/path/perf-baseline.json");
    expect(result).toBeNull();
    expect(warnSpy).toHaveBeenCalled();
    const warnMsg: string = warnSpy.mock.calls[0][0] as string;
    expect(warnMsg).toMatch(/WARNING/);
  });

  it("returns null and emits a warning when the file contains invalid JSON", () => {
    const tmpFile = writeTempText("{ not valid json }");
    try {
      const result = loadBaseline(tmpFile);
      expect(result).toBeNull();
      expect(warnSpy).toHaveBeenCalled();
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("returns the parsed object when the file is valid", () => {
    const baseline = {
      bundleSizeKb: 198.4,
      ttiMs: 1820,
      commitSha: "a3f9c12",
      capturedAt: "2025-01-15T10:30:00.000Z",
    };
    const tmpFile = writeTempJson(baseline);
    try {
      const result = loadBaseline(tmpFile);
      expect(result).not.toBeNull();
      expect(result!.bundleSizeKb).toBe(198.4);
      expect(result!.ttiMs).toBe(1820);
      expect(result!.commitSha).toBe("a3f9c12");
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });
});

// ---------------------------------------------------------------------------
// computeBundleSize helper — sanity checks
// ---------------------------------------------------------------------------

describe("computeBundleSize", () => {
  it("sums an array of chunk sizes correctly", () => {
    expect(computeBundleSize([10, 20, 30])).toBeCloseTo(60, 5);
  });

  it("returns 0 for an empty array", () => {
    expect(computeBundleSize([])).toBe(0);
  });

  it("handles a single chunk", () => {
    expect(computeBundleSize([42.5])).toBeCloseTo(42.5, 5);
  });
});
