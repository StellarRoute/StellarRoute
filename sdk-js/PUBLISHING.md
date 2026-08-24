# Publishing `@stellarroute/sdk-js`

The package is published to npm from a clean checkout of `main`. Everything below runs
from the `sdk-js/` directory unless stated otherwise.

## Versioning policy

- Semantic Versioning. Pre-`1.0`, a **minor** bump may carry breaking changes; a patch
  never does.
- Every breaking change gets a `### Breaking` entry in [`CHANGELOG.md`](./CHANGELOG.md)
  with a before/after snippet.
- The npm `dist-tag` is `latest` for stable releases and `next` for pre-releases
  (`0.2.0-rc.1`).

## Pre-flight checklist

- [ ] On `main`, up to date with the remote, and `git status` is clean.
- [ ] `npm ci` — install from the lockfile, not a stale `node_modules`.
- [ ] `npm run verify` — build (ESM + CJS), unit tests, and example typechecks.
- [ ] `npm run typecheck` passes with no errors.
- [ ] `npm run docs:api` regenerated if public types changed; `docs/sdk-js/api` committed.
- [ ] `CHANGELOG.md` has an entry for this version, with breaking changes called out.
- [ ] README quickstart still compiles against the new types.
- [ ] `npm pack --dry-run` — confirm the tarball contains only `dist/` and `README.md`,
      and that `dist/esm/index.d.ts` plus `dist/cjs/index.js` are present.
- [ ] Node floor in `engines` (`>=18`) still matches what the code actually uses.

## Release

```bash
npm version <patch|minor|major>
```

This writes the new version, commits, and tags. Then:

```bash
npm publish --access public
```

`prepublishOnly` re-runs `npm run verify`, so a broken build cannot be published.

For a pre-release:

```bash
npm version prerelease --preid rc
npm publish --access public --tag next
```

## Post-publish checklist

- [ ] `git push --follow-tags` so the tag reaches the repo.
- [ ] `npm view @stellarroute/sdk-js version` reports the new version.
- [ ] Smoke test in a scratch directory, both module systems:

  ```bash
  npm install @stellarroute/sdk-js
  node -e "import('@stellarroute/sdk-js').then(m => console.log(typeof m.StellarRouteClient))"
  node -e "console.log(typeof require('@stellarroute/sdk-js').StellarRouteClient)"
  ```

- [ ] Cut a GitHub release pointing at the changelog entry.
- [ ] Announce breaking changes to integrators before the tag moves to `latest`.

## Rolling back

Never unpublish. Deprecate the bad version and ship a fix:

```bash
npm deprecate @stellarroute/sdk-js@<bad-version> "Broken build — use <fixed-version>"
npm dist-tag add @stellarroute/sdk-js@<last-good-version> latest
```
