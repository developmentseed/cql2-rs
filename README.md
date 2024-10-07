# CQL2-RS

## WORK IN PROGRESS, NOT READY FOR USE

Parse, validate, and convert CQL2-Text and CQL2-JSON.

## CLI

At its simplest, the command-line interface (CLI) is a pass-through validator:

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

## Response

Responses may not match the input.

### CQL2-Text Differences

- all identifiers in output are double quoted
- position of "NOT" keywords is standardized to be before the expression (ie "... NOT LIKE ..." will become "NOT ... LIKE ..."
- The Negative operator on anything besides a literal number becomes "* -1"
- Parentheses are added around all expressions

Tasks to get to ready-to-use state:
- [x] Parse all examples from CQL2 examples into json that passes json schema validation.
- [x] Add tests that compare OGC examples to parsed/standardized/validated CQL2-Text and CQL2-JSON
- [ ] Fix issues with Z, ZM, and M WKT variants
