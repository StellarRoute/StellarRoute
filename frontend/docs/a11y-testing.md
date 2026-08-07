# Accessibility Testing Baseline

This document defines the axe-core accessibility rules used in the StellarRoute
frontend E2E test suite and the known exclusions that are intentionally deferred.
It is the authoritative reference for the `BASELINE_EXCLUSIONS` array in
`e2e/a11y-swap-flow.spec.ts`.

## Overview

Accessibility scans run as part of the Playwright E2E suite under the `a11y`
project (`playwright.config.ts`). Scans are scoped to the relevant DOM subtree
for each test surface and use `@axe-core/playwright` to enforce WCAG 2.1 AA
conformance.

### Severity tiers

| Impact      | CI behaviour                                       |
|-------------|----------------------------------------------------|
| `critical`  | **Fails the test** — blocks merge                  |
| `serious`   | **Fails the test** — blocks merge                  |
| `moderate`  | Surfaces in report; does **not** block CI          |
| `minor`     | Surfaces in report; does **not** block CI          |

## Surfaces scanned

| # | Surface                             | Scope selector                        |
|---|-------------------------------------|---------------------------------------|
| 1 | Swap form (default, amount, error)  | `[data-testid="swap-card"]`           |
| 2 | Token selection dialog              | `[role="dialog"]`                     |
| 3 | Route list                          | `[data-testid="route-display"]`       |
| 4 | High-impact confirmation modal      | `[role="dialog"]`                     |
| 5 | Settings panel                      | `[data-testid="settings-panel"]`      |
| 6 | Cross-chain deck (`swap_ui_v2`)     | `[data-testid="cross-chain-swap-deck"]` |

## Axe rules in use

The scan runs the full default axe-core rule set, excluding only the rules
listed in [Baseline Exclusions](#baseline-exclusions). All other axe rules are
active and will fail CI on `critical` or `serious` violations.

Commonly exercised rule categories include (but are not limited to):

- **ARIA attributes** (`aria-allowed-attr`, `aria-required-attr`,
  `aria-valid-attr-value`)
- **Accessible names** (`button-name`, `link-name`, `input-label`,
  `aria-label`)
- **Keyboard navigation** (focus order, focus trapping)
- **Landmarks & structure** (`landmark-one-main`, `region`)
- **Color contrast** (`color-contrast` — deferred, see below)
- **Form controls** (`label`, `select-name`)
- **Dialog / modal** (`aria-dialog-name`, `focus-trap`)

## Baseline Exclusions

Rules listed here are explicitly excluded from CI enforcement via the
`BASELINE_EXCLUSIONS` array in `e2e/a11y-swap-flow.spec.ts`. Each entry
documents the rationale and the conditions under which it should be removed.

| # | Rule ID            | Impact  | Date added | Rationale | Removal criteria |
|---|--------------------|---------|------------|-----------|-------------------|
| 1 | `color-contrast`   | serious | 2026-08-07 | The indigo primary palette (`#6366f1`) fails WCAG AA minimum contrast (4.5:1) against the dark-mode backgrounds (`#0a0e1a` / `#141c2b`). This is a design-level colour-token change tracked separately; it must not block unrelated CI. | Redesign the primary colour token to meet WCAG AA contrast against all theme backgrounds, or introduce a high-contrast variant for dark mode. |

## Adding a new exclusion

1. Add the axe rule ID to `BASELINE_EXCLUSIONS` in
   `frontend/e2e/a11y-swap-flow.spec.ts`.
2. Add a comment in the spec explaining why it is deferred.
3. Add a row to the [Baseline Exclusions](#baseline-exclusions) table in this
   document with the rule ID, impact, date, rationale, and removal criteria.
4. Open an issue tracking the removal and link it in the rationale.

## Removing an exclusion

1. Verify the fix resolves the axe violation in all six scanned surfaces.
2. Remove the rule ID from `BASELINE_EXCLUSIONS`.
3. Update the exclusion entry in this document to mark it as **Resolved** with
   the date and PR that removed it.
4. Run the full a11y E2E suite to confirm zero high-severity violations:

   ```bash
   cd frontend
   npx playwright test --project=a11y
   ```

## Running the a11y suite locally

```bash
cd frontend
npx playwright test --project=a11y
```

To run a single test file:

```bash
npx playwright test e2e/a11y-swap-flow.spec.ts --project=a11y
```

## References

- [axe-core rule descriptions](https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md)
- [WCAG 2.1 AA](https://www.w3.org/TR/WCAG21/)
- [Playwright Accessibility Testing](https://playwright.dev/docs/accessibility-testing)
- E2E spec: `frontend/e2e/a11y-swap-flow.spec.ts`
- Related issue: [#312](https://github.com/StellarRoute/StellarRoute/issues/312)
