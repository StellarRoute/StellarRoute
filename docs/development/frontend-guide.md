# Frontend Developer Onboarding Guide

This guide is the entry point for working on the StellarRoute frontend in `frontend/`.

## Prerequisites

- Node.js 18+ (LTS recommended)
- npm 9+
- Running backend API (local or remote)

Install dependencies with a clean lockfile install:

```bash
cd frontend
npm ci
```

Use `npm ci` in local setup and CI to guarantee reproducible installs from `package-lock.json`.

## Local Development

Start the Next.js app:

```bash
cd frontend
npm run dev
```

Frontend runs at `http://localhost:3000` by default.

## Run Frontend Against a Local API

The frontend API client reads `NEXT_PUBLIC_API_URL` and falls back to `http://localhost:8080/api/v1`.

Examples:

```bash
# PowerShell (Windows)
$env:NEXT_PUBLIC_API_URL="http://localhost:8080/api/v1"
npm run dev
```

```bash
# Bash/Zsh
NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1 npm run dev
```

If your backend runs on a different port/host, point `NEXT_PUBLIC_API_URL` to that base API URL.

## Directory Layout

Core frontend directories:

- `frontend/app/`: Next.js App Router routes, layouts, and page-level providers
- `frontend/components/`: UI and feature components (shared, swap, settings, status, etc.)
- `frontend/lib/`: Core frontend utilities and clients (`lib/api`, constants, formatting, feature helpers)

Other useful directories:

- `frontend/hooks/`: Reusable hooks for API polling, swap state, and UI behavior
- `frontend/test/` and `frontend/e2e/`: Unit/integration and Playwright tests

## Testing Workflow (Vitest)

Run frontend tests:

```bash
cd frontend
npm run test
```

Vitest configuration lives in `frontend/vitest.config.ts` and uses a `jsdom` environment.

Important note: `jsdom` does not implement `window.matchMedia`; the project provides a mock in `frontend/vitest.setup.ts`. Keep this in mind when adding tests for responsive or motion-aware components.

## Ladle / Storybook Workflow

StellarRoute uses Ladle for component stories.

- Local stories: `npm run storybook`
- CI build for stories: `npm run storybook:ci`

See `frontend/STORYBOOK.md` for story scope and workflow details.

## Feature Flags

Feature flags are documented in `frontend/src/FEATURE_FLAGS.md`.

Common flag environment variables include:

- `NEXT_PUBLIC_FLAGS_URL`
- `NEXT_PUBLIC_FLAG_<FLAG_NAME>`

Use flags for incremental rollout of experimental UI behavior without redeploying the app.

## Lint, Format, and CI Expectations

Before opening a PR, run:

```bash
cd frontend
npm run lint
npm run format
npm run test
```

CI expectations for frontend contributions:

- Dependencies installed with `npm ci`
- Lint must pass
- Formatting should be clean
- Tests should pass (`vitest`, and where relevant, e2e/storybook checks)

For full contribution workflow, see `CONTRIBUTING.md`.