# CLI

**cql2-rs** includes a command-line interface (CLI).

## Installation

Install from [PyPI](https://pypi.org/project/cql2/):

```shell
pip install cql2
```

Or, if you have [Rust](https://rustup.rs/):

```shell
cargo install cql2-cli
```

## Usage

At its simplest, the CLI is a pass-through validator:

```shell
$ cql2 < tests/fixtures/text/example01.txt # will succeed if the CQL2 is valid
("landsat:scene_id" = 'LC82030282019133LGN00')
```

You can convert formats:

```shell
$ cql2 -o json < tests/fixtures/text/example01.txt
{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}
```

Use `-v` to get detailed validation information:

```shell
$ cql2 'wrong' -v
[ERROR] Invalid CQL2: wrong
For more detailed validation information, use -vv
jsonschema validation failed with file:///tmp/cql2.json#
- at '': oneOf failed, none matched
  - at '': missing properties 'op', 'args'
  - at '': missing properties 'op', 'args'
  - at '': oneOf failed, none matched
    - at '': missing properties 'op', 'args'
    - at '': missing properties 'op', 'args'
    - at '': missing properties 'op', 'args'
    - at '': missing properties 'op', 'args'
    - at '': missing properties 'op', 'args'
  - at '': missing properties 'op', 'args'
  - at '': missing properties 'op', 'args'
  - at '': missing properties 'op', 'args'
  - at '': missing properties 'op', 'args'
  - at '': want boolean, but got object
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
  = expected NotFlag, And, Or, ConcatInfixOp, Add, Subtract, Multiply, Divide, Modulo, Power, Eq, Gt, GtEq, Lt, LtEq, NotEq, Is, or IsNullPostfix
```

Use `cql2 --help` to get a complete listing of the CLI arguments and formats.
