# Contributing to **cql2-rs**

First off, thanks for contributing!
We appreciates you.

## Testing

We aim for comprehensive unit testing of this library.
Please provide tests for any new features, or to demonstrate bugs.
Draft pull requests with a failing test to demonstrate a bug are much appreciated.

## Running the tests

Get [Rust](https://rustup.rs/) and [uv](https://docs.astral.sh/uv/). Then:

```shell
uv sync
scripts/test   # cargo test (the crate and the CLI), then the Python package and its tests
scripts/lint   # rustfmt, clippy, mypy, ruff
```

`cargo test` downloads a prebuilt libduckdb for the DuckDB tests (see `.cargo/config.toml`), so
the first run needs network access.

| Suite | How |
|---|---|
| Rust, every crate | `cargo test --workspace` |
| DuckDB | part of `cargo test`: `operators_duckdb` and `sql_precedence` |
| Longer property tests | `PROPTEST_CASES=100000 cargo test --release --test proptest_roundtrip` |
| Golden files | `tests/generate-expected` rewrites `tests/expected/*.out` and `tests/generate_expected_qcounts` rewrites `tests/operators_expected.txt` (needs `jq`), both from this crate's own output, so they make any change pass: check every changed line and say in the pull request why it changed |
| Python | `uv run maturin develop --uv && uv run pytest` |
| WebAssembly | `cd wasm && wasm-pack test --node` (or `--headless --firefox`), then `wasm-pack build --target nodejs wasm && npm --prefix wasm test` from the root |
| Docs | `scripts/buildwasm && uv run mkdocs serve` |

## Benchmarks

`benches/cql2.rs` times parsing, `reduce`, `filter`, `to_text` and `to_sql` on the
fixtures in `examples/` and `tests/`, and on a few long generated filters. Run
`cargo bench --bench cql2 -- --save-baseline before` on the base branch, then
`cargo bench --bench cql2 -- --baseline before` with your change to compare. Pass a name, such
as `cargo bench --bench cql2 -- reduce/`, to run only some. Benchmarks build with the release
profile.

## Submitting changes

Please open a [pull request](https://docs.github.com/en/pull-requests) with your changes -- make sure to include unit tests.
Please follow standard git commit formatting (subject line 50 characters max, wrap the body at 72 characters).

If you can, use `git rebase -i` to create a clean, well-formatted history before opening your pull request.
If you need to make changes after opening your pull request (e.g. to fix CI breakages) we will be grateful if you squash those fixes into their relevant commits.

Thanks so much!
