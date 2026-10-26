# Tests

The top level `examples` directory is copied directly from <https://github.com/opengeospatial/ogcapi-features/tree/cql2-1.0.0/cql2/standard/schema/examples>.

## Expected test output

Expected files should end with ".out". The first line of each test should be the input in either CQL2 Text or CQL2 JSON. The second line is the expected output in CQL2 Text. The third line is the expected output in CQL2 JSON. All input should be formatted to fit in a single line.

To regenerate the expected output:

```shell
tests/generate-expected
```

This rewrites lines two and three of each existing `.out` file from the input on line one. Adding a
new case means creating the file with its input line first.

## Operator expectations

`operators_tests.txt` lists one query per line. `operators_expected.txt` gives each query followed
by the `intfield` of every row of `cql2testdata.ndjson` it selects. It is written by this crate's
own CLI (`--filter`, needs `jq`), so check every line it changes and say why:

```shell
tests/generate_expected_qcounts
```

## Reduction fixtures

`reductions.txt` holds pairs of lines: an expression in cql2-text, then what it reduces to against
the item in `reduce_tests.rs`.

`proptest_roundtrip.rs` checks generated expressions. Set `PROPTEST_CASES` for a longer run.

## OGC conformance tests

The Abstract Test Suite's expressions, in `ats/expressions.txt`, include the value-to-value
comparisons of `/conf/property-property/comparison-value-value`, which are also evaluated.
