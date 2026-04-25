# API Versioning and Deprecation Policy

This policy governs all HTTP endpoints under /api.

## Versioning model

- Path versioning is used: /api/v1, /api/v2, etc.
- Minor additive changes are allowed within a version:
  - adding optional fields
  - adding optional query parameters
  - adding new endpoints
- Breaking changes require a new major API version.

## Deprecation lifecycle

When a version is deprecated:

- Responses include:
  - Deprecation: true
  - Sunset: <RFC 7231 date>
  - Link: <migration guide>; rel="deprecation"
- The sunset date is announced before breaking removal.
- Existing fields are not silently removed before the notice period ends.

## Breaking-change rules

- No removal or type change of an existing response field in a live version without a published deprecation window.
- No required query/path parameter changes in-place for a live version.
- If behavior must change incompatibly, publish the change in a new major version and provide migration steps.

## Integrator expectations

- Integrators should track deprecation headers and plan migration before sunset.
- Integrators should pin API version paths explicitly in clients.
