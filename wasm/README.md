# cql2-wasm

A small [WebAssembly](https://webassembly.org/) module for parsing and translating [CQL2](https://www.ogc.org/standards/cql2/).

## Usage

Add to your project:

```sh
npm i cql2-wasm
```

Then:

```js
import init, { Expr } from 'cql2-wasm'

// The published package is built with `wasm-pack --target web`, which does not
// initialize itself, so await the default export before using the API.
await init()

const expr = new Expr('collection = foo')

// Throws if the expression is not valid CQL2.
expr.validate()

// `to_json()` returns a JSON *string*; use `JSON.parse` if you want an object.
console.log('to_json():', expr.to_json())
console.log('to_text():', expr.to_text())
```

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

This package includes two types of tests:

### Rust Unit Tests

Unit tests are written in `tests/common/mod.rs` and support execution in various environments.

> [!NOTE]
> All demonstrated commands are to be run from the root of the repository

#### Firefox

Run `tests/web.rs` browser tests in a WASM environment using `wasm-bindgen-test`:

```shell
wasm-pack test --firefox wasm
```

Then, open <http://127.0.0.1:8000/> to see the test(s) run.

#### Node

Run `tests/node.rs` browser tests in a WASM environment using `wasm-bindgen-test`:

```shell
wasm-pack test --node wasm
```

### JavaScript Integration Tests

These tests are written in JavaScript and verify the actual JavaScript API surface that developers will use.
The tests work with both nodejs and web targets:

```shell
# Test with nodejs target
wasm-pack build --target nodejs wasm
npm --prefix wasm test

# Test with web target
wasm-pack build --target web wasm
npm --prefix wasm test
```
