# Dependency Audit Gate — Exception Process

**Issue:** #997  
**Milestone:** M5 — Security

---

## Overview

The `dependency-audit` CI workflow runs `cargo audit` (Rust) and
`npm audit` (frontend, sdk-js) on every PR and daily on `main`. Any
advisory **not** on an accepted-exception list will fail the build.

This document explains how to add a new exception, the criteria for
acceptance, and how to review exceptions over time.

---

## Cargo Audit Exceptions (Rust)

Accepted Rust advisories are listed in `.cargo/audit.toml` under
`[advisories] ignore`. Each entry must carry an inline comment with:

1. The advisory ID (e.g. `RUSTSEC-2023-0071`).
2. The affected crate and why it is a transitive dependency we cannot remove.
3. A brief justification for why the risk is acceptable (no fix available,
   feature flag ensures the vulnerable code path is unreachable, etc.).
4. A review date by which the exception must be re-evaluated.

**Example entry:**

```toml
[advisories]
ignore = [
    # RUSTSEC-2023-0071 | rsa: Marvin Attack timing sidechannel.
    # Transitive dep of sqlx-mysql; project uses only the postgres feature
    # so the mysql crate is never compiled into the binary.
    # No fix available upstream. Re-evaluate: 2027-01-01.
    "RUSTSEC-2023-0071",
]
```

### Adding a New Cargo Exception

1. Confirm no patch exists in the advisory or upstream crate.
2. Determine whether the vulnerable code path is reachable in this project.
3. Open a PR that adds the advisory ID and comment to `.cargo/audit.toml`.
4. Get a second approval from a maintainer before merging.
5. Create a follow-up issue titled
   `[dependency] Re-evaluate RUSTSEC-YYYY-XXXX by <review-date>` and
   assign it to the next milestone.

---

## npm Audit Exceptions (frontend / sdk-js)

`npm audit --audit-level=high` fails only on **high** and **critical**
advisories. Moderate and below are logged but do not block CI.

For a high-severity advisory that cannot be resolved by a package update:

1. Confirm you have run `npm audit fix` and assessed whether a breaking
   `--force` update is safe.
2. Document the advisory in the table below (this file), with the same
   fields: ID, affected package, reason, reachable path, review date.
3. If an `npm audit` flag (`--ignore-script`, `overrides` in package.json)
   is needed to suppress it, it must be reviewed by a maintainer.

### Accepted npm Exceptions

| Advisory | Package | Severity | Reason | Review Date |
|---|---|---|---|---|
| GHSA-frvp-7c67-39w9 | `@hono/node-server <2.0.5` via `shadcn >=3.8.4` | Moderate | The fix requires downgrading `shadcn` to `3.8.3`, which is a **breaking change** to the component generator CLI (dev-only tool, not shipped in production bundles). `@hono/node-server` is a transitive dep of `shadcn`'s MCP server feature and is never executed at runtime or in CI tests. Risk is Windows-only path traversal in `serve-static`, not reachable in our usage. Re-evaluate when `shadcn >=3.8.4` ships a non-breaking fix. | 2026-10-01 |

---

## Scheduled Reviews

- Exceptions are reviewed during the planning cycle for each milestone.
- The daily scheduled CI run will flag any advisory that has since received
  a fix; at that point the exception **must** be removed and the dependency
  updated.
- The exception table in this file is the source of truth for npm
  exceptions. `.cargo/audit.toml` is the source of truth for Rust.

---

## Criteria for Rejection

An exception request will be rejected if:

- A patched version of the affected package exists and is compatible.
- The vulnerable code path is reachable in normal operation.
- The advisory is rated **critical** and no mitigating factor is documented.
- The PR lacks a second maintainer approval.

---

## References

- `.cargo/audit.toml` — Rust advisory ignore list
- `.github/workflows/dependency-audit.yml` — CI gate definition
- `audit/known-issues.md` — accepted product-level risks (separate from
  dependency CVEs)
- [RustSec Advisory Database](https://rustsec.org/)
- [npm Security Advisories](https://github.com/advisories)
