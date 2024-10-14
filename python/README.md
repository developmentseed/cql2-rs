# cql2

Python library and command-line interface (CLI) for parsing and converting [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/), with Rust under the hood.

## Usage

```shell
pip install cql2
```

Then:

```python
expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
# or
expr = cql2.parse_file("fixtures/text/example01.txt")

s = expr.to_text()
d = expr.to_json()
sql = expr.to_sql()
print("SQL query:", sql.query)
print("SQL params:", sql.params)
```

Or from via the command-line interface:

```shell
$ cql2 -o json "landsat:scene_id = 'LC82030282019133LGN00'"
{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}
```

## Developing

To install the package to your virtual environment and test:

```shell
maturin develop --uv -m python/Cargo.toml && pytest python
```

## More information

This package is part of [cql2-rs](https://github.com/developmentseed/cql2-rs/), see that repo for license and contributing information.
