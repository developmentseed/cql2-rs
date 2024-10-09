# cql2-rs

**cql2-rs** is command-line interface (CLI), Python package, and Rust crate for parsing, validating, and converting [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/).

## CLI

To install the CLI, you'll need [Rust](https://rustup.rs/).
Then:

```shell
$ cargo install cql2-cli
$ cql2 -o json "landsat:scene_id = 'LC82030282019133LGN00'"
{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}
```

## Python

Install with **pip**:

```shell
python -m pip install cql2
```

Then:

```python
from cql2 import Expr
expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
d = expr.to_json()
```

See [the API documentation](./python.md) for more.

## Rust

Add **cql2** to your dependencies:

```shell
cargo add cql2
```

Then:

```rust
use cql2::Expr;

let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse().unwrap();
assert!(expr.is_valid());
let json = expr.to_json().unwrap();
```

See [the API documentation](https://docs.rs/cql2) for more.
