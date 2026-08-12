# Releasing

Releases are automated with [release-plz](https://release-plz.dev/).
There is no manual version bump, no hand-edited changelog release section, and no manual `git tag`.

## How it works

1. **Land your changes on `main`.**
   Use [Conventional Commits](https://www.conventionalcommits.org/) for commit and PR titles, and record user-visible changes under `## [Unreleased]` in [CHANGELOG.md](./CHANGELOG.md).

2. **release-plz opens a release PR.**
   Every push to `main` runs the `release-plz-pr` job in [`.github/workflows/release-plz.yml`](./.github/workflows/release-plz.yml).
   It opens (or updates) a release PR that bumps the version in `[workspace.package]`, updates the inter-crate dependency requirements, and moves the `Unreleased` changelog entries under the new version heading.
   Review it like any other PR; if the generated changelog needs editing, push to that branch.

3. **Merge the release PR.**
   The resulting push to `main` runs the `release-plz-release` job, which:
   - publishes `cql2` and `cql2-cli` to crates.io (`cql2-python` and `cql2-wasm` are `publish = false`),
   - creates the tags `cql2-vX.Y.Z` and `cql2-cli-vX.Y.Z`,
   - creates the matching GitHub releases.

4. **The `cql2-vX.Y.Z` tag fans out to the other publishers.**
   - [`pypi.yml`](./.github/workflows/pypi.yml) builds the wheels and sdist and publishes to [PyPI](https://pypi.org/project/cql2/).
   - [`npm.yml`](./.github/workflows/npm.yml) builds the WASM package (`--target web`) and publishes to [npm](https://www.npmjs.com/package/cql2-wasm).
   - [`docs.yml`](./.github/workflows/docs.yml) deploys the versioned documentation with `mike`.

## Notes

- **Do not create tags by hand.** All three tag-triggered workflows match `cql2-v*`. A hand-cut `vX.Y.Z` tag matches none of them, and tagging manually alongside release-plz produces a split release where some artifacts ship and others do not.
- All four packages share a single version, defined once in `[workspace.package]` in the root `Cargo.toml`.
- Each of the three workflows above also accepts `workflow_dispatch` for re-runs. `pypi.yml` and `npm.yml` only publish when the dispatch targets a tag ref; `docs.yml` will deploy the most recent `cql2-v*` tag.
