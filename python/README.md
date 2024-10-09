# cql2

Python library for parsing and converting [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/), with Rust under the hood.

## Usage

```shell
pip install cql2
```

Then:

```python
expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
# or
expr = Expr.from_path("fixtures/text/example01.txt")

s = expr.to_text()
d = expr.to_json()
sql = expr.to_sql()
print("SQL query:", sql.query)
print("SQL params:", sql.params)
```

## Developing

To install the package to your virtual environment and test:

```shell
maturin develop --uv -m python/Cargo.toml && pytest python
```

## More information

This package is part of [cql2-rs](https://github.com/developmentseed/cql2-rs/), see that repo for license and contributing information.