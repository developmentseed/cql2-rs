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
