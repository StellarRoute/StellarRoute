# Swap visual regression workflow

This project uses Playwright snapshot assertions in `frontend/e2e/visual-baseline.spec.ts` for swap-critical UI coverage.

## Coverage matrix

- Themes: light + dark
- Breakpoints: mobile (375x812), tablet (768x1024), desktop (1280x960)
- States:
  - idle swap screen
  - route summary after entering an amount
  - wallet connect CTA state

## CI behavior

- CI runs `npm run test:visual` in the `frontend-visual-regression` job.
- Snapshot drift causes `toHaveScreenshot` failures and fails the job.

## Updating baselines intentionally

1. Make your UI change.
2. Run:
   - `npm --prefix frontend run test:visual:update`
3. Review modified files under `frontend/e2e/visual-baseline.spec.ts-snapshots/`.
4. Commit both the UI change and updated snapshots in the same PR.

## Flake mitigation

- Animations and transitions are disabled in test setup.
- Tests wait briefly after hydration before screenshots.
- CI retries once for Playwright tests.
- Visual assertions use `maxDiffPixelRatio` (2%) configured in `playwright.config.ts`.

