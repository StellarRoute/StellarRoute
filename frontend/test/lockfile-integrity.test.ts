/**
 * Lockfile Integrity — Bug Condition Exploration Test
 *
 * **Property 1: Bug Condition** — Lockfile Root Manifest Drift
 *
 * This test is a BUGFIX EXPLORATION test. It is EXPECTED TO FAIL on unfixed
 * code. Failure confirms the bug exists (lockfile root manifest does not match
 * package.json). The test will pass once the lockfiles are regenerated.
 *
 * **Validates: Requirements 1.1, 1.2, 1.3**
 *
 * isBugCondition(pkg_dir):
 *   manifest_deps != lockfile_root_deps  →  bug exists
 *
 * The desired property (P) is:
 *   For every package directory, the lockfile root manifest EXACTLY matches
 *   the package.json dependencies + devDependencies (same keys, same version
 *   ranges). When this property holds, `npm ci` will succeed.
 */

import { describe, it, expect } from "vitest";
import * as fc from "fast-check";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const __dirname = path.dirname(fileURLToPath(import.meta.url));
// Workspace root is two levels up from frontend/test/
const WORKSPACE_ROOT = path.resolve(__dirname, "..", "..");

interface PackageJson {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
}

interface PackageLockJson {
  packages: {
    "": {
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
  };
}

/**
 * Read and merge dependencies + devDependencies from a package.json file.
 * Returns a flat map of { packageName → versionRange }.
 */
function readManifestDeps(pkgJsonPath: string): Record<string, string> {
  const raw = fs.readFileSync(pkgJsonPath, "utf8");
  const pkg: PackageJson = JSON.parse(raw);
  return {
    ...(pkg.dependencies ?? {}),
    ...(pkg.devDependencies ?? {}),
  };
}

/**
 * Read and merge dependencies + devDependencies from the root "" entry of a
 * package-lock.json (v3 format) file.
 * Returns a flat map of { packageName → versionRange }.
 */
function readLockfileRootDeps(lockfilePath: string): Record<string, string> {
  const raw = fs.readFileSync(lockfilePath, "utf8");
  const lock: PackageLockJson = JSON.parse(raw);
  const root = lock.packages[""];
  return {
    ...(root.dependencies ?? {}),
    ...(root.devDependencies ?? {}),
  };
}

/**
 * Compute the symmetric difference between two dependency maps.
 * Returns an object describing:
 *   - inManifestNotLock: keys present in package.json but absent from lockfile root
 *   - inLockNotManifest: keys present in lockfile root but absent from package.json
 *   - versionMismatches: keys present in both but with different version ranges
 */
function computeDrift(
  manifestDeps: Record<string, string>,
  lockfileDeps: Record<string, string>
): {
  inManifestNotLock: string[];
  inLockNotManifest: string[];
  versionMismatches: Array<{ name: string; manifest: string; lockfile: string }>;
} {
  const inManifestNotLock = Object.keys(manifestDeps).filter(
    (k) => !(k in lockfileDeps)
  );
  const inLockNotManifest = Object.keys(lockfileDeps).filter(
    (k) => !(k in manifestDeps)
  );
  const versionMismatches = Object.keys(manifestDeps)
    .filter((k) => k in lockfileDeps && manifestDeps[k] !== lockfileDeps[k])
    .map((k) => ({ name: k, manifest: manifestDeps[k], lockfile: lockfileDeps[k] }));

  return { inManifestNotLock, inLockNotManifest, versionMismatches };
}

// ---------------------------------------------------------------------------
// Bug Condition Exploration — Concrete checks on the two failing directories
// ---------------------------------------------------------------------------

describe("lockfile integrity — bug condition exploration", () => {
  /**
   * Property 1: Bug Condition — frontend/ lockfile root manifest must exactly
   * match frontend/package.json.
   *
   * EXPECTED TO FAIL on unfixed code because @next/bundle-analyzer and
   * @playwright/test are present in package.json but absent from the lockfile
   * root manifest.
   *
   * **Validates: Requirements 1.1, 1.3**
   */
  it("frontend/package-lock.json root manifest matches frontend/package.json (bug condition check)", () => {
    const pkgJsonPath = path.join(WORKSPACE_ROOT, "frontend", "package.json");
    const lockfilePath = path.join(WORKSPACE_ROOT, "frontend", "package-lock.json");

    const manifestDeps = readManifestDeps(pkgJsonPath);
    const lockfileDeps = readLockfileRootDeps(lockfilePath);

    const drift = computeDrift(manifestDeps, lockfileDeps);

    // Surface the counterexample clearly in the failure message
    const driftSummary = JSON.stringify(drift, null, 2);

    expect(drift.inManifestNotLock, `Entries in package.json missing from lockfile root:\n${driftSummary}`).toEqual([]);
    expect(drift.inLockNotManifest, `Entries in lockfile root missing from package.json:\n${driftSummary}`).toEqual([]);
    expect(drift.versionMismatches, `Version range mismatches between package.json and lockfile root:\n${driftSummary}`).toEqual([]);
  });

  /**
   * Property 1: Bug Condition — sdk-js/ lockfile root manifest must exactly
   * match sdk-js/package.json.
   *
   * **Validates: Requirements 1.2**
   */
  it("sdk-js/package-lock.json root manifest matches sdk-js/package.json (bug condition check)", () => {
    const pkgJsonPath = path.join(WORKSPACE_ROOT, "sdk-js", "package.json");
    const lockfilePath = path.join(WORKSPACE_ROOT, "sdk-js", "package-lock.json");

    const manifestDeps = readManifestDeps(pkgJsonPath);
    const lockfileDeps = readLockfileRootDeps(lockfilePath);

    const drift = computeDrift(manifestDeps, lockfileDeps);

    const driftSummary = JSON.stringify(drift, null, 2);

    expect(drift.inManifestNotLock, `Entries in package.json missing from lockfile root:\n${driftSummary}`).toEqual([]);
    expect(drift.inLockNotManifest, `Entries in lockfile root missing from package.json:\n${driftSummary}`).toEqual([]);
    expect(drift.versionMismatches, `Version range mismatches between package.json and lockfile root:\n${driftSummary}`).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Property-based exploration — isBugCondition holds for any drifted state
// ---------------------------------------------------------------------------

describe("lockfile integrity — property-based drift detection", () => {
  /**
   * Property 1 (PBT): For any set of dependency entries that are present in
   * package.json but absent from the lockfile root manifest, the drift
   * detection function correctly identifies them as missing.
   *
   * This property encodes the isBugCondition check: if manifest_deps !=
   * lockfile_root_deps, the bug condition holds and `npm ci` will fail.
   *
   * **Validates: Requirements 1.1, 1.2, 1.3**
   */
  it("computeDrift detects all missing entries (property test)", () => {
    // Validates: Requirements 1.1, 1.2, 1.3
    fc.assert(
      fc.property(
        // Generate a base set of deps that are in both manifest and lockfile
        fc.dictionary(
          fc.string({ minLength: 1, maxLength: 50 }).filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s)),
          fc.string({ minLength: 1, maxLength: 20 }).filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 0, maxKeys: 10 }
        ),
        // Generate extra deps that are ONLY in the manifest (simulating drift)
        fc.dictionary(
          fc.string({ minLength: 1, maxLength: 50 })
            .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s))
            .map((s) => `@drift/${s}`),
          fc.string({ minLength: 1, maxLength: 20 }).filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 1, maxKeys: 5 }
        ),
        (baseDeps, driftDeps) => {
          // manifest has both base + drift entries
          const manifestDeps = { ...baseDeps, ...driftDeps };
          // lockfile only has base entries (simulating the bug condition)
          const lockfileDeps = { ...baseDeps };

          const drift = computeDrift(manifestDeps, lockfileDeps);

          // Every drift entry must appear in inManifestNotLock
          for (const key of Object.keys(driftDeps)) {
            expect(drift.inManifestNotLock).toContain(key);
          }

          // No base entries should appear in inManifestNotLock
          for (const key of Object.keys(baseDeps)) {
            expect(drift.inManifestNotLock).not.toContain(key);
          }
        }
      ),
      { numRuns: 50 }
    );
  });

  /**
   * Property 1 (PBT): When manifest and lockfile root are identical, drift
   * detection reports no drift (the bug condition does NOT hold).
   *
   * **Validates: Requirements 1.1, 1.2, 1.3**
   */
  it("computeDrift reports no drift when manifest and lockfile root are identical (property test)", () => {
    // Validates: Requirements 1.1, 1.2, 1.3
    fc.assert(
      fc.property(
        fc.dictionary(
          fc.string({ minLength: 1, maxLength: 50 }).filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s)),
          fc.string({ minLength: 1, maxLength: 20 }).filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 0, maxKeys: 15 }
        ),
        (deps) => {
          // Both manifest and lockfile have the same entries — no drift
          const drift = computeDrift(deps, deps);

          expect(drift.inManifestNotLock).toEqual([]);
          expect(drift.inLockNotManifest).toEqual([]);
          expect(drift.versionMismatches).toEqual([]);
        }
      ),
      { numRuns: 50 }
    );
  });
});

// ---------------------------------------------------------------------------
// Preservation Property Tests — Property 2
// ---------------------------------------------------------------------------

/**
 * Preservation Property Tests
 *
 * **Property 2: Preservation** — Non-`npm ci` Workflows Unaffected
 *
 * These tests encode Property 2 (Preservation): for any input where
 * `isBugCondition` is false (manifest_deps == lockfile_root_deps), the
 * `computeDrift` function correctly reports no drift. Conversely, for any
 * drifted state, `computeDrift` correctly fires.
 *
 * These tests MUST PASS on unfixed code — they test the `computeDrift` logic
 * itself, not the actual committed lockfiles.
 *
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
 */
describe("lockfile integrity — preservation property tests", () => {
  /**
   * Property 2.1: Gate stays silent for all clean (non-drifted) states.
   *
   * For any arbitrary set of dependency entries that are reflected in BOTH
   * package.json and the lockfile root manifest (isBugCondition is false),
   * computeDrift must return no drift in any category.
   *
   * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
   */
  it("computeDrift stays silent for all clean (non-drifted) dependency states (preservation property)", () => {
    // Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6
    fc.assert(
      fc.property(
        // Generate arbitrary dependency maps — same map used for both
        // manifest and lockfile, so isBugCondition is false by construction
        fc.dictionary(
          fc
            .string({ minLength: 1, maxLength: 60 })
            .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s)),
          fc
            .string({ minLength: 1, maxLength: 25 })
            .filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 0, maxKeys: 20 }
        ),
        (deps) => {
          // Both manifest and lockfile have identical entries — clean state
          const drift = computeDrift(deps, deps);

          expect(
            drift.inManifestNotLock,
            "Clean state must produce no inManifestNotLock entries"
          ).toEqual([]);
          expect(
            drift.inLockNotManifest,
            "Clean state must produce no inLockNotManifest entries"
          ).toEqual([]);
          expect(
            drift.versionMismatches,
            "Clean state must produce no versionMismatches"
          ).toEqual([]);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Property 2.2: Gate fires for all drifted states.
   *
   * For any arbitrary drift scenario where entries are present in package.json
   * but absent from the lockfile root manifest, computeDrift must detect every
   * missing entry in inManifestNotLock.
   *
   * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
   */
  it("computeDrift fires for all drifted states (entries in package.json not in lockfile) (preservation property)", () => {
    // Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6
    fc.assert(
      fc.property(
        // Base deps present in both manifest and lockfile
        fc.dictionary(
          fc
            .string({ minLength: 1, maxLength: 60 })
            .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s))
            .map((s) => `base-${s}`),
          fc
            .string({ minLength: 1, maxLength: 25 })
            .filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 0, maxKeys: 10 }
        ),
        // Drift deps present ONLY in manifest (simulating package.json edited
        // without re-running npm install — the exact root cause of this bug)
        fc.dictionary(
          fc
            .string({ minLength: 1, maxLength: 60 })
            .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s))
            .map((s) => `drift-${s}`),
          fc
            .string({ minLength: 1, maxLength: 25 })
            .filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 1, maxKeys: 10 }
        ),
        (baseDeps, driftDeps) => {
          // Manifest has base + drift; lockfile only has base
          const manifestDeps = { ...baseDeps, ...driftDeps };
          const lockfileDeps = { ...baseDeps };

          const drift = computeDrift(manifestDeps, lockfileDeps);

          // Every drift entry must be detected
          for (const key of Object.keys(driftDeps)) {
            expect(
              drift.inManifestNotLock,
              `Drift entry "${key}" must be detected in inManifestNotLock`
            ).toContain(key);
          }

          // The total count must match exactly
          expect(drift.inManifestNotLock.length).toBe(
            Object.keys(driftDeps).length
          );

          // Base entries must NOT appear as drifted
          for (const key of Object.keys(baseDeps)) {
            expect(drift.inManifestNotLock).not.toContain(key);
          }

          // No false positives in the other direction
          expect(drift.inLockNotManifest).toEqual([]);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Property 2.3: computeDrift correctly handles version range mismatches.
   *
   * When the same dependency key exists in both manifest and lockfile but with
   * different version ranges, computeDrift must report it as a versionMismatch
   * (not as missing from either side).
   *
   * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
   */
  it("computeDrift correctly handles version range mismatches (same key, different version) (preservation property)", () => {
    // Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6
    fc.assert(
      fc.property(
        // A shared dep name present in both
        fc
          .string({ minLength: 1, maxLength: 60 })
          .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s))
          .map((s) => `pkg-${s}`),
        // Two distinct version strings for the same dep
        fc
          .string({ minLength: 1, maxLength: 25 })
          .filter((s) => /^[\^~]?\d/.test(s))
          .map((v) => `1.${v}`),
        fc
          .string({ minLength: 1, maxLength: 25 })
          .filter((s) => /^[\^~]?\d/.test(s))
          .map((v) => `2.${v}`),
        // Additional shared deps with matching versions (no drift)
        fc.dictionary(
          fc
            .string({ minLength: 1, maxLength: 60 })
            .filter((s) => /^[a-z@][a-z0-9@/._-]*$/.test(s))
            .map((s) => `shared-${s}`),
          fc
            .string({ minLength: 1, maxLength: 25 })
            .filter((s) => /^[\^~]?\d/.test(s)),
          { minKeys: 0, maxKeys: 8 }
        ),
        (mismatchedKey, manifestVersion, lockfileVersion, sharedDeps) => {
          // Versions are guaranteed different by the 1.x / 2.x prefix
          const manifestDeps = {
            ...sharedDeps,
            [mismatchedKey]: manifestVersion,
          };
          const lockfileDeps = {
            ...sharedDeps,
            [mismatchedKey]: lockfileVersion,
          };

          const drift = computeDrift(manifestDeps, lockfileDeps);

          // The mismatched key must appear in versionMismatches
          const mismatch = drift.versionMismatches.find(
            (m) => m.name === mismatchedKey
          );
          expect(
            mismatch,
            `Version mismatch for "${mismatchedKey}" must be reported`
          ).toBeDefined();
          expect(mismatch?.manifest).toBe(manifestVersion);
          expect(mismatch?.lockfile).toBe(lockfileVersion);

          // The mismatched key must NOT appear as missing from either side
          expect(drift.inManifestNotLock).not.toContain(mismatchedKey);
          expect(drift.inLockNotManifest).not.toContain(mismatchedKey);

          // Shared deps must not produce any drift
          for (const key of Object.keys(sharedDeps)) {
            expect(drift.inManifestNotLock).not.toContain(key);
            expect(drift.inLockNotManifest).not.toContain(key);
          }
        }
      ),
      { numRuns: 100 }
    );
  });
});
