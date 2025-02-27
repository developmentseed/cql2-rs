# cql2-wasm

This is a no-release crate to build a small [WebAssembly](https://webassembly.org/) wrapper for this crate.

## Building

Get [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).
Then (from the top-level directory in this repo):

```shell
scripts/buildwasm
```

You can then preview our WASM playground locally.
Get [uv](https://docs.astral.sh/uv/getting-started/installation/), then:

```shell
uv sync
uv run mkdocs serve
```

The playground will be available at <http://127.0.0.1:8000/cql2-rs/playground/>.
There is a live version available at <http://developmentseed.org/cql2-rs/latest/playground/>.

## Testing

```shell
wasm-pack test --firefox wasm
```

Then, open <http://127.0.0.1:8000/> to see the test(s) run.

## Releasing to NPM

```shell
wasm-pack build wasm
# TODO actually release
```
