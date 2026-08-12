# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Every item marked **Breaking** changes the result of code that compiles unchanged.

### Changed

- **Breaking:** `to_text` emits the minimum parentheses needed rather than wrapping every operation. `((a = 1) AND ((b = 2) OR (c = 3)))` is now `a = 1 AND (b = 2 OR c = 3)`. The expressions are equivalent, but any consumer comparing `to_text` output as a string will see different results
- **Breaking:** `to_sql` and `to_ducksql` output changes wherever grouping, a timestamp literal, or a date bound is involved. See the corresponding entries under Fixed
- **Breaking:** `Error` is now `#[non_exhaustive]`, so adding a variant will no longer be a breaking change
- **Breaking:** `sqlparser` 0.58 → 0.62. This is a public dependency — `ToSqlAst::to_sql_ast` returns `sqlparser::ast::Expr` — so anything naming that type must move too
- **Breaking:** the `TEMPORALOPS` and `ARRAYOPS` constants carry the spelling the JSON schema defines, so `TEMPORALOPS.contains(&"t_metby")` is now `false`; use `"t_metBy"`. `temporal_op` accepts either spelling
- **Breaking:** deserializing an `Expr` normalizes it, so cql2-json no longer round-trips byte for byte: `and`/`or` chains flatten, operator names take the schema's spelling, and timestamps are canonicalized. Expressions that used to compare unequal may now compare equal. This is `Expr`'s `Deserialize` impl rather than any one entry point, so `parse_json`, `Expr::try_from(Value)`, `serde_json::from_str::<Expr>`, an `Expr` field on a `#[derive(Deserialize)]` struct and the bindings' mapping constructors all agree
- **Breaking:** `eq` is no longer accepted as an alias for `=`. CQL2 defines no such operator and the grammar never had one, so `eq(a, b)` is a user-defined function call and is now preserved as written. `!=` remains an accepted spelling of `<>`, because the grammar does define it

### Fixed

- **Security:** `to_sql` and `to_ducksql` could be made to emit a string literal that terminates early, letting a crafted property value inject arbitrary SQL. `textfield = 'x\'' OR 1=1 --'` generated a predicate matching every row. Quotes in a literal are now escaped before the value reaches the SQL printer
- `BETWEEN` no longer discards the rest of the expression. `a BETWEEN 1 AND 2 AND b = 3` and `b = 3 AND a BETWEEN 1 AND 2` each silently parsed to just the `BETWEEN`. It is now a grammar production over scalar operands, so the `AND` delimiting its bounds is not confused with the boolean connective ([#255](https://github.com/developmentseed/cql2-rs/issues/255))
- `to_sql` and `to_ducksql` parenthesize sub-expressions that bind more loosely than their parent, so `A AND (B OR C)` is no longer rendered as the inequivalent `A AND B OR C` ([#255](https://github.com/developmentseed/cql2-rs/issues/255))
- `to_text` no longer drops grouping in arithmetic: `(a + b) * c` was rendered as `a + b * c`, which parses back as a different expression
- Input the grammar cannot consume in full is a parse error. Previously the parse ended at the last complete atom and the remainder was discarded, so a filter quietly meant something narrower than what was written
- A property name beginning with a keyword is no longer misread. `notes = 1` parsed as `NOT (es = 1)` and passed validation; `null_count`, `true_color` and `false_positive` were truncated to the keyword
- `s_intersects(geom, BBOX(...))` and the other spatial predicates evaluate against a bounding box. A geometry and a bbox were treated as incomparable operand kinds, so the predicate never folded and every row was rejected without error
- `INTERVAL('..', t)` and `INTERVAL(t, '..')` evaluate. The specification admits `..` for an unbounded bound, and two of the shipped examples use it; the predicate silently matched nothing and the SQL rendering was rejected by the database
- A date means the whole day in the SQL backends as it always has in the evaluator, so the two no longer disagree at a day boundary. A bare literal compared against a date still asks whether the two name the same day
- `t_overlaps` and `t_overlappedBy` require the earlier range to begin first. Their first conjunct compared one range's start to the other's end, which is implied by the rest of the condition, so they also matched ranges that are wholly contained in one another
- A one-element array survives `to_text`. `a_overlaps(x, ['a'])` rendered as `a_overlaps(x, ('a'))`, which re-parsed as a scalar and then matched nothing
- Geometries no longer lose their third ordinate, in either direction, and a three-dimensional `GEOMETRYCOLLECTION` renders to text that parses back. Evaluating one previously aborted the process
- A string literal containing an apostrophe round-trips, and a quoted identifier may contain a quote. `O'Brien` rendered as `'O''Brien'`, which the grammar then rejected
- A leap second is no longer rewritten to a different instant during timestamp canonicalization
- `reduce` returns errors instead of panicking on malformed input: wrong operand counts for `isNull`, `not`, `casei`, `accenti` and `between`, and `and`/`or` over two geometry-bearing operands, all aborted the process
- Malformed expressions return errors instead of panicking across `to_text` and `to_sql`, including wrong operand counts for every operator and exponent-form numbers such as `1e5`, which previously reached an `unreachable!`
- `cql2 --filter` fails on an evaluation error instead of silently dropping the row it happened on
- The function spelling `in(a, 1, 2)` produces the same expression as the infix `a IN (1, 2)`; it previously produced a flat argument list from which the renderers kept only the first item
- The function spelling `isNull(a)` produces the same operator name as the postfix `a IS NULL`
- `a OR b OR c` parses to one n-ary `or`, matching the cql2-json encoding and the existing handling of `and`
- Timestamp literals are canonical, so `TIMESTAMP('2012-08-10T05:30:00.000000Z')` and `TIMESTAMP('2012-08-10T05:30:00Z')` produce equal expressions
- Operator names parsed from cql2-text carry the spelling the JSON encoding requires, so `T_METBY(..)` yields `t_metBy`. Names CQL2 does not define are function names and keep the case the author wrote, in both `to_text` and `to_sql`
- `to_text` quotes a function name only where the cql2-text grammar requires it, rather than applying PostgreSQL's rule
- String literals render in cql2-text form rather than PostgreSQL's `E'...'` escape-string syntax, which cql2-text cannot read back
- `NOT LIKE` and `NOT IN` no longer depend on there being exactly one space between the words
- `\r` counts as whitespace, so a CRLF-terminated expression parses
- `div` is preserved as the integer-division operator CQL2 defines
- `t_disjoint` renders as a self-delimiting SQL predicate, so `isNull(t_disjoint(..))` is no longer regrouped by the database
- `and`, `or`, `not` and the comparisons follow the three-valued logic CQL2 and SQL share. A NULL operand collapsed the whole operation to `false`, so `null OR true` was `false` rather than `true`; NULL now propagates except where the other operand already decides the answer — `FALSE AND anything` is FALSE and `TRUE OR anything` is TRUE. `filter` and `matches` admit a record only when the predicate is TRUE, so a NULL answer excludes it without raising an error. An operand that merely could not be evaluated yet — an unresolved property, a function call — is still left unfolded rather than being read as NULL
- `div` is evaluated. It was listed as an arithmetic operator but had no implementation, so `5 div 2` reduced to itself. It is integer division, truncated toward zero as PostgreSQL and the SQL standard require, so `5 div 2` is 2 and `-5 div 2` is -2. Division by zero has no integer answer and is left unfolded rather than yielding the infinity `/` gives
- `to_text` reports an error for an infinity or a NaN instead of writing the bare words `inf` and `NaN`, which cql2-text has no literals for and reads back as *property* names: `1 / 0` rendered as `inf`, an expression that parses and means something else. `to_sql` is unaffected, and still writes `CAST('Infinity' AS DOUBLE)`
- `to_text` requires two operands for `+`, `-`, `*`, `/` and `%`, as `to_sql` already did. One operand rendered as that operand alone, so `{"op":"-","args":[{"property":"a"}]}` became `a` and the operator vanished
- `a_equals` renders as a set comparison in SQL, matching the evaluator, which compares two arrays as sets. It rendered as `=`, which is positional in both PostgreSQL and DuckDB, so `a_equals(intarrayfield, (3,2,1))` was true in the evaluator and false in the database for a row holding `[1,2,3]`
- `LIKE` is emitted with an explicit `ESCAPE '\'`, so every backend reads the pattern the way the evaluator does. DuckDB has no default escape character, which made `like(textfield, 'item\_1')` select a disjoint set of rows there
- A nested `GEOMETRYCOLLECTION` is a parse error naming the reason, rather than a function call named `GEOMETRYCOLLECTION` that the schema then accepts. The cql2-json encoding admits only the six non-collection geometry types as members, so a nested collection has no CQL2 expression
- `EMPTY` geometries parse, for every geometry type. GeoJSON with no coordinates is rendered as `POLYGON EMPTY`, which the grammar had no production for, so this crate's own output did not parse back

### Other

- Operator precedence is defined once, in `precedence`, and shared by the cql2-text parser, `to_text` and the SQL backends
- `NOTICE` and `examples/NOTICE` record the OGC material this repository redistributes and the modifications made to it, and `src/cql2.json` carries the same attribution
- Added `tests/encoding_invariants.rs` (round-tripping and cross-encoding agreement without reference to the golden files), `tests/sql_precedence.rs` (a filter-versus-DuckDB differential), `tests/temporal_relations.rs` (each temporal relation against its definition), `tests/ats_conformance.rs` (the 109 filter expressions named by the OGC CQL2 Abstract Test Suite) and `tests/proptest_roundtrip.rs` (generated expressions), with `proptest` as a new dev-dependency
- Both expectation generators rebuild their output atomically: a query the CLI rejects aborts the run with the expectations untouched, where previously a broken invocation blanked every one of them
- `examples/examples.toml` records the same expected output as the golden files, and a test keeps the two in step

## [0.5.7](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.6...cql2-v0.5.7) - 2026-07-29

### Fixed

- parse bbox into expr ([#246](https://github.com/developmentseed/cql2-rs/pull/246))
- fix null reductions ([#232](https://github.com/developmentseed/cql2-rs/pull/232))

### Other

- *(deps)* bump the production-dependencies group across 1 directory with 13 updates ([#251](https://github.com/developmentseed/cql2-rs/pull/251))
- *(deps)* bump the github-actions group across 1 directory with 3 updates ([#250](https://github.com/developmentseed/cql2-rs/pull/250))
- fix libduckdb download w/ cache ([#252](https://github.com/developmentseed/cql2-rs/pull/252))
- *(deps)* bump the production-dependencies group with 9 updates ([#244](https://github.com/developmentseed/cql2-rs/pull/244))
- *(deps)* bump the github-actions group across 1 directory with 4 updates ([#242](https://github.com/developmentseed/cql2-rs/pull/242))
- *(deps)* bump the production-dependencies group across 1 directory with 8 updates ([#243](https://github.com/developmentseed/cql2-rs/pull/243))
- *(deps)* bump the production-dependencies group with 7 updates ([#239](https://github.com/developmentseed/cql2-rs/pull/239))
- *(deps)* bump the production-dependencies group across 1 directory with 8 updates ([#236](https://github.com/developmentseed/cql2-rs/pull/236))
- *(deps)* bump astral-sh/setup-uv in the github-actions group ([#233](https://github.com/developmentseed/cql2-rs/pull/233))
- *(deps)* bump the production-dependencies group with 9 updates ([#234](https://github.com/developmentseed/cql2-rs/pull/234))
- *(deps)* bump the production-dependencies group with 6 updates ([#230](https://github.com/developmentseed/cql2-rs/pull/230))
- *(deps)* bump actions/create-github-app-token ([#228](https://github.com/developmentseed/cql2-rs/pull/228))
- *(deps)* bump the production-dependencies group with 3 updates ([#229](https://github.com/developmentseed/cql2-rs/pull/229))
- *(deps)* bump the production-dependencies group with 4 updates ([#226](https://github.com/developmentseed/cql2-rs/pull/226))

### Fixed

- `reduce` no longer constant-folds predicates whose value is unknown: `IS NULL`, `IN`, `BETWEEN`, and comparisons over an unresolved property are now preserved instead of being folded to an incorrect boolean ([#231](https://github.com/developmentseed/cql2-rs/issues/231), [#111](https://github.com/developmentseed/cql2-rs/issues/111))
- negative number literals now parse to negative literals instead of being expanded as `-1 * n` ([#112](https://github.com/developmentseed/cql2-rs/issues/112))

## [0.5.6](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.5...cql2-v0.5.6) - 2026-05-08

### Other

- *(deps)* bump the production-dependencies group with 8 updates ([#224](https://github.com/developmentseed/cql2-rs/pull/224))
- manually bump the geo dependency, re-add dependabot groups ([#223](https://github.com/developmentseed/cql2-rs/pull/223))
- *(deps)* bump web-sys from 0.3.95 to 0.3.97 ([#221](https://github.com/developmentseed/cql2-rs/pull/221))
- *(deps)* bump geozero from 0.14.0 to 0.15.1 ([#222](https://github.com/developmentseed/cql2-rs/pull/222))
- *(deps)* bump jsonschema from 0.33.0 to 0.46.2 ([#212](https://github.com/developmentseed/cql2-rs/pull/212))
- *(deps)* bump geojson from 0.24.2 to 1.0.0 ([#213](https://github.com/developmentseed/cql2-rs/pull/213))
- *(deps)* bump jiff from 0.2.23 to 0.2.24 ([#214](https://github.com/developmentseed/cql2-rs/pull/214))
- *(deps)* bump geo from 0.31.0 to 0.33.1 ([#215](https://github.com/developmentseed/cql2-rs/pull/215))
- *(deps)* bump sqlparser from 0.58.0 to 0.61.0 ([#216](https://github.com/developmentseed/cql2-rs/pull/216))

## [0.5.5](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.4...cql2-v0.5.5) - 2026-04-21

### Fixed

- remove cargo groups ([#205](https://github.com/developmentseed/cql2-rs/pull/205))

### Other

- *(deps)* bump sqlparser from 0.58.0 to 0.61.0 ([#207](https://github.com/developmentseed/cql2-rs/pull/207))
- *(deps)* bump jsonschema from 0.33.0 to 0.46.2 ([#206](https://github.com/developmentseed/cql2-rs/pull/206))
- *(deps)* bump geojson from 0.24.2 to 1.0.0 ([#209](https://github.com/developmentseed/cql2-rs/pull/209))
- *(deps)* bump geozero from 0.14.0 to 0.15.1 ([#210](https://github.com/developmentseed/cql2-rs/pull/210))
- *(deps)* bump geo from 0.31.0 to 0.33.1 ([#208](https://github.com/developmentseed/cql2-rs/pull/208))
- revert the versioning strategy
- versioning-strategy for cargo ([#211](https://github.com/developmentseed/cql2-rs/pull/211))
- update wasm-bindgen in cargo.lock ([#202](https://github.com/developmentseed/cql2-rs/pull/202))

## [0.5.4](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.3...cql2-v0.5.4) - 2026-04-21

### Fixed

- dependabot for cargo ([#200](https://github.com/developmentseed/cql2-rs/pull/200))

### Other

- *(deps)* bump the production-dependencies group with 12 updates ([#201](https://github.com/developmentseed/cql2-rs/pull/201))
- *(deps-dev)* update mkdocs-material requirement ([#192](https://github.com/developmentseed/cql2-rs/pull/192))
- *(deps-dev)* update ruff requirement from >=0.6.9 to >=0.15.10 ([#193](https://github.com/developmentseed/cql2-rs/pull/193))
- *(deps)* bump the github-actions group with 4 updates ([#198](https://github.com/developmentseed/cql2-rs/pull/198))
- dependabot groups ([#197](https://github.com/developmentseed/cql2-rs/pull/197))
- *(deps)* bump actions/setup-node from 6.3.0 to 6.4.0 ([#196](https://github.com/developmentseed/cql2-rs/pull/196))
- *(deps)* bump astral-sh/setup-uv from 7.6.0 to 8.0.0 ([#182](https://github.com/developmentseed/cql2-rs/pull/182))
- pin GitHub Actions to SHA digests ([#181](https://github.com/developmentseed/cql2-rs/pull/181))
- *(deps)* bump actions/create-github-app-token from 2.2.1 to 3.0.0 ([#177](https://github.com/developmentseed/cql2-rs/pull/177))
- *(deps)* bump actions/download-artifact from 7 to 8 ([#171](https://github.com/developmentseed/cql2-rs/pull/171))
- *(deps)* bump actions/upload-artifact from 6 to 7 ([#172](https://github.com/developmentseed/cql2-rs/pull/172))
- *(deps)* bump actions/attest-build-provenance from 3 to 4 ([#173](https://github.com/developmentseed/cql2-rs/pull/173))
- *(deps)* bump the production-dependencies group across 1 directory with 12 updates ([#169](https://github.com/developmentseed/cql2-rs/pull/169))

## [0.5.3](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.2...cql2-v0.5.3) - 2026-02-04

### Fixed

- use flags, not values, for validate and reduce in CLI ([#165](https://github.com/developmentseed/cql2-rs/pull/165))

### Other

- *(deps)* bump the production-dependencies group across 1 directory with 6 updates ([#166](https://github.com/developmentseed/cql2-rs/pull/166))
- *(deps)* bump the production-dependencies group across 1 directory with 4 updates ([#163](https://github.com/developmentseed/cql2-rs/pull/163))
- *(deps)* bump the production-dependencies group across 1 directory with 9 updates ([#161](https://github.com/developmentseed/cql2-rs/pull/161))

## [0.5.2](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.1...cql2-v0.5.2) - 2026-01-16

### Fixed

- one element lists ([#160](https://github.com/developmentseed/cql2-rs/pull/160))

### Other

- *(deps)* bump the production-dependencies group across 1 directory with 6 updates ([#158](https://github.com/developmentseed/cql2-rs/pull/158))

## [0.5.1](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.5.0...cql2-v0.5.1) - 2026-01-05

### Added

- *(wasm)* add to_ducksql ([#156](https://github.com/developmentseed/cql2-rs/pull/156))

### Other

- *(deps)* bump the production-dependencies group across 1 directory with 7 updates ([#155](https://github.com/developmentseed/cql2-rs/pull/155))
- *(deps)* bump the production-dependencies group across 1 directory with 6 updates ([#154](https://github.com/developmentseed/cql2-rs/pull/154))
- *(deps)* bump the production-dependencies group across 1 directory with 4 updates ([#152](https://github.com/developmentseed/cql2-rs/pull/152))
- *(deps)* bump actions/download-artifact from 6 to 7 ([#147](https://github.com/developmentseed/cql2-rs/pull/147))
- *(deps)* bump actions/create-github-app-token from 2.0.6 to 2.2.1 ([#148](https://github.com/developmentseed/cql2-rs/pull/148))
- *(deps)* bump actions/upload-artifact from 5 to 6 ([#149](https://github.com/developmentseed/cql2-rs/pull/149))
- *(deps)* bump the production-dependencies group across 1 directory with 5 updates ([#150](https://github.com/developmentseed/cql2-rs/pull/150))

## [0.5.0](https://github.com/developmentseed/cql2-rs/compare/cql2-v0.4.2...cql2-v0.5.0) - 2025-12-08

### Added

- add __str__ and __repr__ ([#122](https://github.com/developmentseed/cql2-rs/pull/122))
- add __version__ to python module ([#121](https://github.com/developmentseed/cql2-rs/pull/121))

### Fixed

- *(ci)* remove locked check ([#125](https://github.com/developmentseed/cql2-rs/pull/125))
- it wasn't used anywhere and it was broken ([#124](https://github.com/developmentseed/cql2-rs/pull/124))
- *(ci)* update to latest npm for publishing
- *(ci)* remove clean install from npm publish

### Other

- use release bot for releasing ([#145](https://github.com/developmentseed/cql2-rs/pull/145))
- *(deps)* bump the production-dependencies group across 1 directory with 3 updates ([#144](https://github.com/developmentseed/cql2-rs/pull/144))
- fix ci config ([#143](https://github.com/developmentseed/cql2-rs/pull/143))
- *(deps)* bump the production-dependencies group with 3 updates ([#142](https://github.com/developmentseed/cql2-rs/pull/142))
- add release-plz workflow ([#140](https://github.com/developmentseed/cql2-rs/pull/140))
- *(deps)* bump the production-dependencies group with 6 updates ([#138](https://github.com/developmentseed/cql2-rs/pull/138))
- *(deps)* bump actions/checkout from 5 to 6 ([#134](https://github.com/developmentseed/cql2-rs/pull/134))
- try groups ([#136](https://github.com/developmentseed/cql2-rs/pull/136))
- group dependencies ([#135](https://github.com/developmentseed/cql2-rs/pull/135))
- *(ci)* actually keep the locked check ([#127](https://github.com/developmentseed/cql2-rs/pull/127))
- *(deps)* bump actions/setup-node from 4 to 6 ([#126](https://github.com/developmentseed/cql2-rs/pull/126))
- *(wasm)* [**breaking**] Make WASM module interface more like Python module ([#120](https://github.com/developmentseed/cql2-rs/pull/120))
- update cargo lock

### Changed

- Normalized WASM interface to match Python API ([#120](https://github.com/developmentseed/cql2-rs/pull/120))
  - Renamed `CQL2` to `Expr` for consistency with Python
  - Changed `Expr.matches()` and `Expr.reduce()` to accept JS objects instead of strings
  - Changed `Expr.to_json()` to return JS objects instead of strings

## [0.4.2] - 2025-11-12

### Changed

- Better package metadata and automated releasing

## [0.4.1] - 2025-10-31

## Changed

- Deploy abi3 wheels ([#94](https://github.com/developmentseed/cql2-rs/pull/94))

## Removed

- Python 3.9 ([#94](https://github.com/developmentseed/cql2-rs/pull/94))

## [0.4.0] - 2025-09-16

### Changed

- Reworked SQL generation to use `SqlParser` AST as the target.
- Modified `to_ducksql` to leverage SQL AST, only changing aspects specific to DuckDB.

### Added

- `filter` method to filter passed-in JSON values.
- Sample data to run test filters against.
- 155 tests covering every CQL2 operator.
- Test runners for both internal `reduce`/`matches` and DuckDB.
- Exposed `reduce` option in WASM / WASM Playground.

### Fixed

- Numerous issues found with the new tests.

## [0.3.8] - 2025-09-09

### Changed

- Bump some dependencies ([#87](https://github.com/developmentseed/cql2-rs/pull/87))

## [0.3.7] - 2025-03-28

### Added

- Experimental DuckDB SQL ([#70](https://github.com/developmentseed/cql2-rs/pull/70))

## [0.3.6] - 2025-03-27

### Changed

- Update examples ([#75](https://github.com/developmentseed/cql2-rs/pull/75))
- Further reductions for and/or ([#78](https://github.com/developmentseed/cql2-rs/pull/78))

### Added

- Expose `Expr.matches()` in Python ([#76](https://github.com/developmentseed/cql2-rs/pull/76))
- Expose `Expr.reduce()` in Python ([#79](https://github.com/developmentseed/cql2-rs/pull/79))

## [0.3.5] - 2025-03-12

### Fixed

- Timestamp math ([#67](https://github.com/developmentseed/cql2-rs/pull/67))

## [0.3.4] - 2025-02-21

### Added

- Enable combining expressions via addition ([#68](https://github.com/developmentseed/cql2-rs/pull/68))

## [0.3.3] - 2024-02-18

### Added

- WASM ([#59](https://github.com/developmentseed/cql2-rs/pull/59))
- Match cql2 against JSON ([#55](https://github.com/developmentseed/cql2-rs/pull/55))

## [0.3.2] - 2024-12-09

### Fixed

- Packaging ([#51](https://github.com/developmentseed/cql2-rs/pull/51))

## [0.3.1] - 2024-11-14

### Fixed

- Invalid parse while combining AND and OR ([#47](https://github.com/developmentseed/cql2-rs/pull/47))

## [0.3.0] - 2024-10-14

### Changed

- Use free functions (instead of staticmethods) in the Python API ([#41](https://github.com/developmentseed/cql2-rs/pull/41))

## [0.2.0] - 2024-10-10

### Added

- Python bindings 🐍 ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- Docs ([#36](https://github.com/developmentseed/cql2-rs/pull/36))

### Changed

- `SqlQuery` attributes are now public ([#30](https://github.com/developmentseed/cql2-rs/pull/30))
- `Expr::to_json`, `Expr::to_json_pretty`, and `Expr::to_value` now return `Error` instead of `serde_json::Error` ([#37](https://github.com/developmentseed/cql2-rs/pull/37))
- Removed `Error::BoonCompile` ([#38](https://github.com/developmentseed/cql2-rs/pull/38))

## [0.1.0] - 2024-10-08

Initial release.

[Unreleased]: https://github.com/developmentseed/cql2-rs/compare/v0.4.2...main
[0.4.2]: https://github.com/developmentseed/cql2-rs/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/developmentseed/cql2-rs/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/developmentseed/cql2-rs/compare/v0.3.8...v0.4.0
[0.3.8]: https://github.com/developmentseed/cql2-rs/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/developmentseed/cql2-rs/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/developmentseed/cql2-rs/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/developmentseed/cql2-rs/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/developmentseed/cql2-rs/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/developmentseed/cql2-rs/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/developmentseed/cql2-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/developmentseed/cql2-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/developmentseed/cql2-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/developmentseed/cql2-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/developmentseed/cql2-rs/tag/v0.1.0

<!-- markdownlint-disable-file MD024 -->
