# Cargo.lock policy (StellarRoute)

Root `Cargo.lock` is listed in `.gitignore` (line 4). This is intentional for the workspace:

- Library/application crates resolve dependencies at CI build time via `Cargo.toml` version pins.
- Contributors run `cargo build` / `cargo test` which generates a local lockfile; it must not be committed.
- When adding workspace dependencies (e.g. `alloy-primitives`, `alloy-sol-types` 0.8 with minimal default features), pin exact compatible versions in the relevant `Cargo.toml` files.
- CI determinism is enforced through pinned versions in manifests and stable Rust toolchain, not a tracked root lockfile.

If project policy changes to track `Cargo.lock`, remove it from `.gitignore` and commit the lockfile in the same PR as dependency additions.
