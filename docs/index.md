# cql2-rs

**cql2-rs** is [Python package](./python.md), [command-line interface (CLI)](./cli.md), and [Rust crate](https://docs.rs/cql2) for parsing, validating, and converting [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/).

## Python

```python
>>> from cql2 import Expr
>>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
>>> expr.to_json()
{'op': '=', 'args': [{'property': 'landsat:scene_id'}, 'LC82030282019133LGN00']}
```

## CLI

```shell
$ cql2 < tests/fixtures/text/example01.txt # will succeed if the CQL2 is valid
("landsat:scene_id" = 'LC82030282019133LGN00')
```

## Rust

```rust
use cql2::Expr;
let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse();
let json = expr.to_json().unwrap();
```
