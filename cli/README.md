# cql2-cli

A Command Line Interface (CLI) for [Common Query Language (CQL2)](https://www.ogc.org/standard/cql2/).

## Installation

With cargo:

```shell
cargo install cql2-cli
```

Or from [PyPI](https://pypi.org/project/cql2/):

```shell
pip install cql2
```

## CLI

At its simplest, the CLI is a pass-through validator:

```shell
$ cql2 < examples/text/example01.txt # will succeed if the CQL2 is valid
landsat:scene_id = 'LC82030282019133LGN00'
```

You can convert formats:

```shell
$ cql2 -o json < examples/text/example01.txt
{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}
```

Use `-v` to get detailed validation information:

```shell
$ cql2 'wrong' -v
[ERROR] Invalid CQL2: wrong
For more detailed validation information, use -vv
{"property":"wrong"} is not valid under any of the schemas listed in the 'oneOf' keyword
```

cql2-text parsing errors are pretty-printed:

```shell
$ cql2 '(foo ~= "bar")'
[ERROR] Parsing error: (foo ~= "bar")
 --> 1:6
  |
1 | (foo ~= "bar")
  |      ^---
  |
  = expected NotFlag, And, Or, Add, Subtract, Multiply, Divide, Modulo, Power, Eq, Gt, GtEq, Lt, LtEq, NotEq, or IsNullPostfix
```

Use `cql2 --help` to get a complete listing of the CLI arguments and formats.

## More information

See [the top-level README](https://github.com/developmentseed/cql2-rs/blob/main/README.md) for license and contributing information.
