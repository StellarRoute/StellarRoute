# Storybook / Ladle for StellarRoute frontend

This repository uses Ladle to render component stories for core swap primitives.

## Available stories
- `Swap primitives` (Core components in `components/swap/`)
- `TokenSelector` (Shared component)
- `QuoteCard` (Shared component)
- `RouteRow` (Shared component)
- `SlippageControl` (Shared component)

## Run locally
1. `cd frontend`
2. `npm install`
3. `npm run storybook`

## CI command
- `npm run storybook:ci`

The CI workflow step is in `.github/workflows/ci.yml` under the `frontend` job.
