# Releasing

Setup:

- Install [cargo-release](https://github.com/crate-ci/cargo-release): `cargo install cargo-release`

Then:

1. Create a new branch: `release/vX.Y.Z`
2. Update the version in `Cargo.toml`
3. Update the CHANGELOG
4. Update each README 
5. Open a PR
6. Once approved, merge the PR
7. (if releasing the main crate) Run `cargo release -p cql2`, then `cargo release -p cql2 --execute`
8. (if releasing the cli crate) run `cargo release -p cql2-cli`, then `cargo release -p cql2-cli --execute`
