# Contributing to StellarRoute

Thank you for your interest in StellarRoute! 🌟 We're building critical infrastructure for the Stellar ecosystem and welcome contributors of **all skill levels** — from first-time open-source contributors to seasoned Rust engineers.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style & Standards](#code-style--standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Commit Message Conventions](#commit-message-conventions)
- [Issue Reporting](#issue-reporting)
- [Good First Contributions](#good-first-contributions)
- [Communication Channels](#communication-channels)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating you agree to uphold a welcoming, respectful environment for everyone. Please report unacceptable behaviour to the maintainers via a GitHub Discussion.

---

## Getting Started

### Prerequisites

| Tool                    | Version | Notes                                   |
| ----------------------- | ------- | --------------------------------------- |
| Rust                    | 1.75+   | Install via [rustup](https://rustup.rs) |
| Docker & Docker Compose | Latest  | Local PostgreSQL & Redis                |
| Git                     | 2.x+    |                                         |
| Node.js                 | 18+     | Frontend only                           |

### 1. Fork & Clone

```bash
# Fork the repository first (GitHub UI), then:
git clone https://github.com/<your-username>/StellarRoute.git
cd StellarRoute
```

### 2. Start Local Services

```bash
docker-compose up -d
```

This starts:

- **PostgreSQL 15** on `localhost:5432`
- **Redis 7** on `localhost:6379`

### 3. Build the Project

```bash
cargo build
```

### 4. Run Tests

```bash
cargo test
```

### 5. Run the API Server (optional)

```bash
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute \
  cargo run -p stellarroute-api
```

The API will be available at `http://localhost:3000`. Visit `http://localhost:3000/swagger-ui` for interactive API docs.

For a more detailed environment setup, see [docs/development/SETUP.md](docs/development/SETUP.md).

---

## Development Workflow

### Branching Strategy

| Branch            | Purpose                         |
| ----------------- | ------------------------------- |
| `main`            | Stable, always passing CI       |
| `feature/<topic>` | New features or enhancements    |
| `fix/<topic>`     | Bug fixes                       |
| `docs/<topic>`    | Documentation-only changes      |
| `chore/<topic>`   | Tooling, CI, dependency updates |

**Always branch off `main`:**

```bash
git checkout main
git pull origin main
git checkout -b feature/my-feature
```

### Keeping Your Branch Up to Date

```bash
git fetch origin
git rebase origin/main
```

Prefer rebasing over merging to keep a clean history.

---

## Code Style & Standards

### Rust

- Run `cargo fmt` before every commit — CI will reject unformatted code.
- Run `cargo clippy -- -D warnings` and fix all lints.
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for public APIs.
- Use `thiserror` for library error types and `anyhow` for application-level errors.
- Prefer `tracing` over `println!` / `eprintln!` for logging.
- Document public items with `///` doc comments.

```bash
# Handy pre-commit check
cargo fmt && cargo clippy -- -D warnings && cargo test
```

### General

- Keep functions and modules small and focused.
- Write self-documenting code; add comments only when _why_ isn't obvious.
- Avoid `unwrap()` in production paths — use `?` or explicit error handling.
- No `unsafe` code (enforced by workspace lint).

---

## Testing Requirements

Every code contribution **must** include appropriate tests.

| Change type          | Required tests                                                               |
| -------------------- | ---------------------------------------------------------------------------- |
| New endpoint         | Integration test (can be `#[ignore]` with DB) + unit test for response model |
| Bug fix              | Regression test that would have caught the bug                               |
| New utility / helper | Unit tests covering happy path and edge cases                                |
| Refactor             | Existing tests must continue to pass                                         |

### Running Tests

```bash
# All unit tests (no external deps needed)
cargo test

# Include ignored integration tests (requires running DB)
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute \
  cargo test -- --include-ignored

# A single crate
cargo test -p stellarroute-api
```

Integration tests that require a live database should be marked with `#[ignore = "requires DATABASE_URL"]` and documented accordingly (see `crates/api/tests/` for examples).

---

## Pull Request Process

### Before Opening a PR

- [ ] Your branch is rebased on the latest `main`
- [ ] `cargo fmt` passes with no changes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all non-ignored tests)
- [ ] New or updated tests are included
- [ ] No `todo!()` / `unimplemented!()` / debug `println!` left in production paths

### Opening the PR

1. Push your branch to your fork.
2. Open a PR against `StellarRoute:main`.
3. Fill in the PR template:
   - Summary of changes
   - Motivation / linked issue (e.g. `Closes #42`)
   - Testing performed
4. Request a review — maintainers aim to respond within **48 hours**.

### Review Checklist (for reviewers)

- [ ] Code is correct and handles errors gracefully
- [ ] Tests are meaningful and cover edge cases
- [ ] Public APIs are documented
- [ ] No unnecessary complexity introduced
- [ ] Follows project conventions (naming, module structure)
- [ ] CI is green

### After Feedback

Push additional commits to your branch; the PR will update automatically. Avoid force-pushing after a review has started unless asked to squash/rebase.

---

## Commit Message Conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

[optional body]

[optional footer: "Closes #<issue>"]
```

### Types

| Type       | When to use                             |
| ---------- | --------------------------------------- |
| `feat`     | New feature                             |
| `fix`      | Bug fix                                 |
| `docs`     | Documentation only                      |
| `refactor` | Code restructuring, no behaviour change |
| `test`     | Adding or fixing tests                  |
| `chore`    | Tooling, CI, dependency bumps           |
| `perf`     | Performance improvement                 |

### Scope (optional)

Use the crate or area affected: `api`, `indexer`, `routing`, `contracts`, `sdk`, `ci`, `db`.

### Examples

```
feat(api): implement GET /api/v1/pairs endpoint

Query distinct trading pairs from SDEX offers and return
them in the canonical Stellar asset identifier format.

Closes #5
```

```
fix(indexer): retry on transient Horizon 503 responses

Adds exponential backoff (100ms → 5s) for up to 3 retries
before propagating the error.

Closes #22
```

---

## Issue Reporting

### Bug Reports

Please include:

1. **Rust version**: `rustc --version`
2. **Steps to reproduce** (minimal)
3. **Expected vs actual behaviour**
4. **Relevant logs** (set `RUST_LOG=debug` for verbose output)

Use the **Bug Report** issue template on GitHub.

### Feature Requests

- Describe the problem you're solving, not just the solution.
- Link to any related issues or discussions.
- Check the [Roadmap](Roadmap.md) first — your feature may already be planned.

### Security Vulnerabilities

**Do not open a public issue.** Contact the maintainers privately via GitHub's [Security Advisories](../../security/advisories) feature.

---

## Good First Contributions

Not sure where to start? Here are some ideas:

- 🟢 Issues tagged [`good-first-issue`](../../issues?q=label%3Agood-first-issue) — well-scoped tasks with clear requirements
- 🔵 Issues tagged [`beginner-friendly`](../../issues?q=label%3Abeginner-friendly) — minimal project context needed
- 📝 Improve or expand documentation in `docs/`
- 🧪 Add tests for existing, untested code paths
- 🐛 Fix a known bug from the issue tracker

**New to Rust?** That's fine! Start with a documentation or testing issue and work your way up. Maintainers are happy to review Rust code and suggest idiomatic improvements.

**New to Stellar?** Read the [Stellar Developer Docs](https://developers.stellar.org) and the project [README](README.md) to get a feel for the domain, then pick a backend or tooling issue.

---

## Communication Channels

| Channel                                 | Purpose                                      |
| --------------------------------------- | -------------------------------------------- |
| [GitHub Issues](../../issues)           | Bug reports, feature requests, task tracking |
| [GitHub Discussions](../../discussions) | Architecture questions, ideas, Q&A           |
| PR comments                             | Code-specific feedback                       |

When in doubt, **open a Discussion** — there are no silly questions. 🙂

---

**Thank you for contributing to StellarRoute!** Every PR, no matter how small, helps build better infrastructure for the Stellar ecosystem. 🚀

_Built with ❤️ for the Stellar ecosystem_
