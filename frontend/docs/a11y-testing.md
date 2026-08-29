# Accessibility Testing Baseline

## Overview

This document establishes the accessibility (a11y) testing baseline for the StellarRoute frontend. It serves as a reference for contributors to understand how accessibility is tested and to explicitly define areas where remediation is out of scope. In particular, contrast changes on swap-related UI components are explicitly excluded from routine ad-hoc fixes. These issues require a dedicated, flagged issue and design review before being addressed.

## Unit-Level A11y Testing

The frontend currently uses **Vitest** for component-level unit testing.
**Currently, there is no unit-level accessibility testing tool (e.g., `vitest-axe` or `@axe-core/react`) installed or configured in this repository.**

Because no tooling is installed, there are no commands to run for unit-level a11y checks. 
**Future Setup:** To add unit-level a11y testing in the future, we would need to install `vitest-axe` (or a similar tool) and configure it in the Vitest setup files to assert against component renders.

## E2E A11y Testing

The frontend uses **Playwright** for end-to-end (e2e) testing, and `@axe-core/playwright` is installed for automated accessibility scans during e2e runs (see `e2e/status-dashboard.spec.ts` for an example of its usage).

To run the existing e2e accessibility tests locally:

```bash
npm --prefix frontend run test:e2e
```

Or, to run a specific test file that includes an axe scan:
```bash
npm --prefix frontend run test:e2e -- e2e/status-dashboard.spec.ts
```

*Note: E2E accessibility tests are currently run locally as there is no specific GitHub Actions CI job configured to run `axe` checks in `.github/workflows/`.*

## Exclusion Table

The following components and pages are explicitly excluded from strict accessibility enforcement during routine PRs:

| Component / Page | Justification |
|------------------|---------------|
| `SwapCard` / `/swap` page contrast | Contrast issues on the swap UI are documented in [`docs/design/accessibility-contrast-audit.md`](../../docs/design/accessibility-contrast-audit.md). Remediation of these contrast issues is explicitly out of scope for routine contributions. Do not attempt to "fix" contrast here without a dedicated, flagged issue that has undergone design review. |

## Scope Statement

**Swap contrast remediation is explicitly out of scope for routine contributions.** Any attempt to adjust colors, borders, or contrast ratios on the `/swap` page or related `SwapCard` components requires its own dedicated, flagged issue with prior design approval. Please do not submit ad-hoc contrast fixes for these components.
