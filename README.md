# cql2-rs

[![CI](https://github.com/developmentseed/cql2-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/developmentseed/cql2-rs/actions/workflows/ci.yml)

Parse, validate, and convert [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/) text and JSON.

## Usage

### API

```toml
[dependencies]
cql = "0.3"
```

Then:

```rust
use cql2::Expr;

let expr: Expr = "landsat:scene_id = 'LC82030282019133LGN00'".parse().unwrap();
assert!(expr.is_valid());
println!("{}", expr.to_json().unwrap());
```

See [the documentation](https://docs.rs/cql2) for more.

## CLI

See [the cql2-cli README](./cli/README.md) for details.

## Responses

Responses may not match the input.

### cql2-text differences

- All identifiers in output are double quoted
- The position of "NOT" keywords is standardized to be before the expression (ie "... NOT LIKE ..." will become "NOT ... LIKE ..."
- The negative operator on anything besides a literal number becomes "* -1"
- Parentheses are added around all expressions

## Development

See [CONTRIBUTING.md](./CONTRIBUTING.md) for information about contributing to this project.

## License

**cql2-rs** is licensed under the MIT license.
See [LICENSE](./LICENSE) for details.
