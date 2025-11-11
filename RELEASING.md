# Releasing

1. Create a new branch: `release/vX.Y.Z`
2. Update the version in `Cargo.toml`
3. Update the CHANGELOG
4. Update each README
5. Open a PR
6. Once approved, merge the PR
7. `git tag -s vX.Y.Z`
8. `git push origin vX.Y.Z`
9. Create a new [release](https://github.com/developmentseed/cql2-rs/releases) for your tag
