# v1 Migration Guide

This guide helps integrators migrate away from deprecated /api/v1 endpoints.

## What changed in v1 now

- v1 routes now emit deprecation headers:
  - Deprecation
  - Sunset
  - Link
- List endpoints now support pagination for large payload safety:
  - GET /api/v1/pairs
  - GET /api/v1/markets

## Pagination migration

Use cursor pagination where possible.

Request examples:

- First page:
  - /api/v1/pairs?limit=25
- Next page:
  - /api/v1/pairs?limit=25&cursor=25

Rules:

- Default limit is 25.
- Maximum limit is 100.
- Invalid cursor values return HTTP 400.

## Health endpoint migration

- Existing /health remains available.
- New /health/deps provides dependency-focused readiness for orchestration.

## Suggested client behavior

- Read and log deprecation headers.
- Follow Link header guidance for migration timelines.
- Avoid relying on undocumented fields.
- Move to the next major version before the sunset date.
