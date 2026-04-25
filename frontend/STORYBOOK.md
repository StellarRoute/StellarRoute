# Storybook / Ladle for StellarRoute frontend

This repository uses Ladle to render component stories for core swap primitives.

## Available stories
- `TokenSelector`
- `QuoteCard`
- `RouteRow`
- `SlippageControl`

## Run locally
1. `cd frontend`
2. `npm install`
3. `npm run storybook`

## CI command
- `npm run storybook:ci`

The CI workflow step is in `.github/workflows/ci.yml` under the `frontend` job.
