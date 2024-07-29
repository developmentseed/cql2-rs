# CQL2-RS

## WORK IN PROGRESS, NOT READY FOR USE

Parse, validate, and convert CQL2-Text and CQL2-JSON.

## CLI
Both commands take either CQL2-Text or CQL2-JSON on stdin, as a quoted/escaped argument, or interactively. They will return status code 0 on successful validation or status code 1 if there was a problem parsing or validating. Verbosity of the validation errors can be controlled using the CQL2_DEBUG_LEVEL environment variable between 0 and 3.
- cql2json - returns standardized CQL2-JSON
- cql2text - returns standardized CQL2-Text

## Response
Response may not match the input.
### CQL2-Text Differences
- all identifiers in output are double quoted
- position of "NOT" keywords is standardized to be before the expression (ie "... NOT LIKE ..." will become "NOT ... LIKE ..."
- The Negative operator on anything besides a literal number becomes "* -1"
- Parentheses are added around all expressions

Tasks to get to ready-to-use state:
- [x] Parse all examples from CQL2 examples into json that passes json schema validation.
- [ ] Add tests that compare OGC examples to parsed/standardized/validated CQL2-Text and CQL2-JSON
- [ ] Fix issues with Z, ZM, and M WKT variants
