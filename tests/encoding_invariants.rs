//! Invariants that hold independently of the golden files in `tests/expected`.
//!
//! The golden files are regenerated from this crate's own output, so they cannot catch a change
//! that corrupts an expression consistently in both directions. These tests compare the encodings
//! against *each other* instead: a rendering that loses grouping stops round-tripping, and an
//! expression that parses differently from cql2-text than from cql2-json stops agreeing.

use cql2::{Expr, ToSqlAst};
use serde_json::Value;
use sqlparser::{dialect::PostgreSqlDialect, parser::Parser};
use std::{fs, path::Path};

/// Expressions whose meaning depends on grouping, beyond what the OGC examples cover.
const GROUPING_BATTERY: &[&str] = &[
    "a = 1 and (b = 2 or c = 3)",
    "(a = 1 or b = 2) and c = 3",
    "a = 1 or b = 2 and c = 3",
    "(a = 1 or b = 2) and (c = 3 or d = 4)",
    "a = 1 and b = 2 and c = 3",
    "a = 1 or b = 2 or c = 3",
    // Same-op chains nested on the right: the connectives are associative, so the renderers emit no
    // parentheses and the expression only round trips if the parse is flat.
    "(a = 1 and b = 2) and (c = 3 and d = 4)",
    "(a = 1 or b = 2) or (c = 3 or d = 4)",
    "a = 1 and (b = 2 and c = 3)",
    "a = 1 or (b = 2 or c = 3)",
    // Spellings the grammar accepts as aliases, and forms whose operator name is split across
    // whitespace.
    "a != b",
    "a != b and c = 1",
    "3 = foo div 2",
    "a NOT LIKE 'x%'",
    "a NOT  LIKE 'x%'",
    "a NOT IN (1, 2)",
    "a = 1e5",
    "a = -1.5e-3",
    "not (a = 1 or b = 2)",
    "not (a = 1 and b = 2)",
    "not (a = 1) and b = 2",
    "(a + b) * c = 1",
    "a * (b + c) = 1",
    "a - (b - c) = 1",
    "a / (b / c) = 1",
    "a + b * c = 1",
    "(a + b) / (c - d) = 1",
    "-(a + b) = 1",
    "-a ^ 2 = 4",
    "-a + b = 1",
    "a ^ b ^ c = 1",
    "a between 1 and 2",
    "a between 1 and 2 and b = 3",
    "b = 3 and a between 1 and 2",
    "a between 1 and 2 and b between 3 and 4",
    "a between 1 and 2 or b = 3",
    "a not between 1 and 2",
    "a between b + 1 and c * 2",
    "a + b between 1 and 2",
    "a between 'x' and 'y'",
    "d between DATE('2020-01-01') and DATE('2020-02-01')",
    "a in (1, 2)",
    "in(a, 1, 2)",
    "a not in (1, 2)",
    "a in (1, 2) and b = 3",
    "a like 'x' and b = 2",
    "a not like 'x'",
    "casei(a) like 'x' and (b = 1 or c = 2)",
    "a is null",
    "a is not null",
    "isNull(a)",
    "(a = 1 or b = 2) and isNull(c)",
    "1 + 2 > 4",
    "s_intersects(geom, POINT(0 0)) and (b = 1 or c = 2)",
    // Written as function calls in cql2-text but rendered as infix operators in SQL (`@>`, `<@`,
    // `@@`, `=`), so their grouping is decided by the SQL precedence of those operators.
    "a_contains(x, (1,2)) and (b = 1 or c = 2)",
    "not a_contains(x, (1,2))",
    "a_contains(x, (1,2)) or b = 1",
    "(a_contains(x, (1,2)) or b = 1) and c = 2",
    "a_containedby(x, (1,2)) and b = 1",
    "a_overlaps(x, (1,2)) and b = 1",
    "a_equals(x, (1,2)) and b = 1",
    "isNull(a_contains(x, (1,2)))",
];

fn parse(source: &str) -> Expr {
    source
        .parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"))
}

/// A comparable form of an expression, independent of any text or SQL rendering.
fn shape(expr: &Expr) -> Value {
    serde_json::from_str(&expr.to_json().expect("expression should serialize"))
        .expect("to_json should emit valid JSON")
}

fn example_sources(dir: &str, extension: &str) -> Vec<(String, String)> {
    let mut sources: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {dir}: {e}"))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some(extension))
        .map(|path| {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("example files have UTF-8 stems")
                .to_string();
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            (stem, source)
        })
        .collect();
    sources.sort();
    assert!(
        !sources.is_empty(),
        "no {extension} examples found in {dir}"
    );
    sources
}

/// Rendering to cql2-text and parsing the result must yield the same expression.
///
/// A rendering that loses grouping fails here: `(a + b) * c` written as `a + b * c` parses back as a
/// different tree.
#[test]
fn text_rendering_round_trips() {
    let mut cases: Vec<(String, String)> = GROUPING_BATTERY
        .iter()
        .map(|q| ((*q).to_string(), (*q).to_string()))
        .collect();
    cases.extend(example_sources("examples/text", "txt"));

    let mut failures = Vec::new();
    for (name, source) in &cases {
        let expr = parse(source);
        let rendered = expr.to_text().expect("expression should render as text");
        match rendered.parse::<Expr>() {
            Ok(reparsed) if shape(&reparsed) == shape(&expr) => {}
            Ok(reparsed) => failures.push(format!(
                "{name}\n     rendered: {rendered}\n     was: {}\n     now: {}",
                shape(&expr),
                shape(&reparsed)
            )),
            Err(e) => failures.push(format!(
                "{name}\n     rendered: {rendered}\n     error: {e}"
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} expressions changed meaning when rendered to text and parsed back:\n  {}",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}

/// The cql2-text and cql2-json encodings of the same OGC example must parse to the same expression.
///
/// The two encodings are independent inputs, so neither can mask a parser bug in the other.
#[test]
fn text_and_json_encodings_agree() {
    let json: std::collections::HashMap<String, String> = example_sources("examples/json", "json")
        .into_iter()
        .collect();
    let text = example_sources("examples/text", "txt");

    let mut compared = 0;
    let mut diverged = Vec::new();
    for (stem, text_source) in &text {
        let Some(json_source) = json.get(stem) else {
            continue;
        };
        compared += 1;
        let from_text = shape(&parse(text_source));
        let from_json = shape(&parse(json_source));
        if from_text != from_json {
            diverged.push(format!(
                "{stem}\n     from text: {from_text}\n     from json: {from_json}"
            ));
        }
    }

    // A silent drop in coverage would make this test look healthy while checking nothing.
    assert!(
        compared >= 100,
        "expected ~109 examples in both encodings, only compared {compared}"
    );

    assert!(
        diverged.is_empty(),
        "{} of {compared} examples parse differently from text than from json:\n  {}",
        diverged.len(),
        diverged.join("\n  ")
    );
}

/// Input the grammar cannot consume in full is rejected, not silently truncated.
///
/// Truncation is the dangerous failure: the surviving prefix is usually itself valid, so the
/// schema validator accepts it and the caller gets a filter that quietly means something narrower
/// than what they wrote.
#[test]
fn malformed_input_is_rejected() {
    const MALFORMED: &[&str] = &[
        // `BETWEEN` whose bounds do not parse; the remainder used to be discarded.
        "a = 1 and b between 3 and not c and d = 2",
        "a = 1 and b between 2 or c = 3",
        "a between 1 and 2 between 3 and 4",
        "a is null between 1 and 2",
        "a = 1 and b between 1",
        "a between 1",
        // `IS` is not an infix operator: CQL2 defines none, and accepting one invented the
        // operation `is(a, b)`, which the schema then waved through as a function call. Only the
        // postfix `IS [NOT] NULL` predicate spells `IS`.
        "a is b",
        "a is not b",
        "a is 1",
        // Trailing text that belongs to no production.
        "a = 1 garbage",
        "a = 1 and",
        "a = 1)",
        "a || b = 'x'",
    ];
    let accepted: Vec<&str> = MALFORMED
        .iter()
        .filter(|source| source.parse::<Expr>().is_ok())
        .copied()
        .collect();
    assert!(
        accepted.is_empty(),
        "these malformed expressions parsed instead of erroring: {accepted:?}"
    );
}

/// Evaluating a geometry must not depend on how many ordinates it carries.
///
/// Spatial predicates convert through WKT, and a rendering that omits the dimension tag produces
/// text no WKT reader accepts — which surfaces as a failure inside `filter`/`matches`, not as a
/// rendering difference any golden file would show.
#[test]
fn geometries_evaluate_at_every_dimension() {
    let items: Vec<Value> = vec![
        serde_json::json!({"id": 1, "geom": {"type": "Point", "coordinates": [1.0, 2.0]}}),
        serde_json::json!({"id": 2, "geom": {"type": "Point", "coordinates": [1.0, 2.0, 3.0]}}),
    ];
    let selected = |expr: &Expr| -> Vec<i64> {
        expr.filter(&items)
            .expect("expression evaluates")
            .iter()
            .map(|item| item["id"].as_i64().expect("id is an integer"))
            .collect()
    };

    // Which rows a geometry selects, not merely that it evaluates. The predicates are planar, so
    // both fixture points sit at (1, 2) whatever their z: selection pins the 2-D answer, while the
    // third ordinate is policed by the round trip below and by
    // `geojson_geometries_render_as_parseable_wkt`. The line y = x misses (1, 2).
    for (source, expected) in [
        ("s_intersects(geom, POINT(1 2))", vec![1, 2]),
        ("s_intersects(geom, POINT Z(1 2 3))", vec![1, 2]),
        (
            "s_intersects(geom, POLYGON((0 0,4 0,4 4,0 4,0 0)))",
            vec![1, 2],
        ),
        (
            "s_intersects(geom, POLYGON Z((0 0 1,4 0 1,4 4 1,0 4 1,0 0 1)))",
            vec![1, 2],
        ),
        ("s_intersects(geom, LINESTRING Z(0 0 1,4 4 2))", vec![]),
    ] {
        let expr = parse(source);
        assert_eq!(
            selected(&expr),
            expected,
            "{source} selected the wrong rows"
        );

        // The same geometry re-read from either rendering must select the same rows. A rendering
        // that loses the third ordinate, or emits one no reader accepts, fails here.
        for rendered in [
            expr.to_text().expect("renders as text"),
            expr.to_json().expect("renders as json"),
        ] {
            assert_eq!(
                selected(&parse(&rendered)),
                expected,
                "{source} rendered as {rendered}"
            );
        }
    }
}

/// A geometry that arrives as GeoJSON keeps its third ordinate when rendered as cql2-text.
///
/// The WKT a cql2-text expression carries is echoed back verbatim, so only a GeoJSON-sourced
/// geometry exercises the conversion that has to tag the dimension.
#[test]
fn geojson_geometries_render_as_parseable_wkt() {
    for (geojson, wkt) in [
        (r#"{"type":"Point","coordinates":[1.0,2.0,3.0]}"#, "POINT Z"),
        (
            r#"{"type":"LineString","coordinates":[[0.0,0.0,1.0],[4.0,4.0,2.0]]}"#,
            "LINESTRING Z",
        ),
        (
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1.0,2.0,3.0]},{"type":"Point","coordinates":[4.0,5.0,6.0]}]}"#,
            "GEOMETRYCOLLECTION Z",
        ),
    ] {
        let source = format!(r#"{{"op":"s_intersects","args":[{{"property":"geom"}},{geojson}]}}"#);
        let expr = parse(&source);
        let text = expr.to_text().expect("renders as text");
        assert!(text.contains(wkt), "expected {wkt} in {text}");
        assert_eq!(
            shape(&parse(&text)),
            shape(&expr),
            "{text} did not parse back to the same geometry"
        );
    }
}

/// An empty geometry survives a round trip from GeoJSON through cql2-text and back.
///
/// GeoJSON with no coordinates renders as the WKT `EMPTY` form — `POLYGON EMPTY` — which the
/// grammar has to read back, or this crate emits text it cannot itself parse.
#[test]
fn empty_geometries_round_trip_from_geojson() {
    // Every GeoJSON geometry that can hold no coordinates. A GeoJSON `Point` cannot: a position is
    // two or more numbers, so an empty one does not deserialize, and `POINT EMPTY` is therefore
    // only reachable from cql2-text (covered below).
    for geojson in [
        r#"{"type":"LineString","coordinates":[]}"#,
        r#"{"type":"Polygon","coordinates":[]}"#,
        r#"{"type":"MultiPoint","coordinates":[]}"#,
        r#"{"type":"MultiLineString","coordinates":[]}"#,
        r#"{"type":"MultiPolygon","coordinates":[]}"#,
        r#"{"type":"GeometryCollection","geometries":[]}"#,
    ] {
        let source = format!(r#"{{"op":"s_intersects","args":[{{"property":"geom"}},{geojson}]}}"#);
        let expr = parse(&source);
        let text = expr.to_text().expect("renders as text");
        assert!(
            text.to_uppercase().contains("EMPTY"),
            "{geojson} rendered as {text}, which is not the EMPTY form"
        );
        assert_eq!(
            shape(&parse(&text)),
            shape(&expr),
            "{text} did not parse back to the same geometry"
        );
    }
}

/// Every geometry type has an `EMPTY` form, and each one parses and renders as itself.
///
/// Compared as text rather than as JSON: an empty *point* has no GeoJSON encoding at all, since a
/// position is two or more numbers, so `to_json` cannot represent it and neither can the schema.
#[test]
fn empty_geometries_parse_in_text() {
    for wkt in [
        "POINT EMPTY",
        "LINESTRING EMPTY",
        "POLYGON EMPTY",
        "MULTIPOINT EMPTY",
        "MULTILINESTRING EMPTY",
        "MULTIPOLYGON EMPTY",
        "GEOMETRYCOLLECTION EMPTY",
        // Written in the case the grammar's other keywords accept, and with a dimension marker.
        "polygon empty",
        "POINT Z EMPTY",
        // As a member of a collection, beside a geometry that has coordinates.
        "GEOMETRYCOLLECTION(POINT EMPTY, POINT(1 2))",
        "GEOMETRYCOLLECTION(POINT(1 2), POLYGON EMPTY)",
    ] {
        let source = format!("s_intersects(geom, {wkt})");
        let expr = parse(&source);
        let text = expr.to_text().expect("renders as text");
        assert!(
            text.to_uppercase().contains("EMPTY"),
            "{wkt} rendered as {text}, which lost the EMPTY"
        );
        let reparsed = parse(&text).to_text().expect("renders as text");
        assert_eq!(reparsed, text, "{text} did not render as itself again");
    }

    // `EMPTY` ends where a name would, so it is not the prefix of a longer word.
    assert!("s_intersects(geom, POINT EMPTYISH)"
        .parse::<Expr>()
        .is_err());
}

/// A `GEOMETRYCOLLECTION` inside a `GEOMETRYCOLLECTION` is refused, and says why.
///
/// WKT allows the nesting; the cql2-json schema gives a collection's members as the six
/// non-collection geometry types, so there is no CQL2 expression for a nested one. The grammar
/// matches it anyway, because the alternative was falling through to the function-call production
/// and silently producing `{"op":"GEOMETRYCOLLECTION", ...}` — an operation that no longer means
/// what was written and that the schema then accepts as a user-defined function.
#[test]
fn nested_geometry_collections_are_refused() {
    for wkt in [
        "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(POINT(0 0), POINT(1 1)), POINT(2 2))",
        "GEOMETRYCOLLECTION(POINT(2 2), GEOMETRYCOLLECTION(POINT(0 0), POINT(1 1)))",
        "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION EMPTY, POINT(2 2))",
    ] {
        let source = format!("s_intersects(geom, {wkt})");
        let error = source
            .parse::<Expr>()
            .expect_err("a nested collection has no CQL2 encoding");
        assert!(
            matches!(error, cql2::Error::NestedGeometryCollection),
            "{wkt} was rejected as {error}, not as a nested collection"
        );
        // The message names the problem rather than the grammar rule that failed.
        let message = error.to_string();
        assert!(
            message.contains("GEOMETRYCOLLECTION"),
            "the message does not name the operator: {message}"
        );
    }

    // A collection of ordinary geometries is unaffected.
    let expr = parse("s_intersects(geom, GEOMETRYCOLLECTION(POINT(0 0), POINT(1 1)))");
    assert!(expr.is_valid());
}

/// A measure ordinate survives the cql2-text rendering, on a collection member as well as on a
/// whole geometry.
///
/// GeoJSON has no measure, so `to_wkt` echoes an `M` geometry back rather than re-rendering it
/// through GeoJSON. A member of a collection carries its marker on its own, past the outermost tag,
/// and dropping it would silently take the third ordinate off *every* member: the collection is then
/// re-rendered at the dimension of its shallowest position.
#[test]
fn measured_geometries_keep_their_ordinates_in_text() {
    for wkt in [
        "POINT M(1 2 3)",
        "POINT ZM(1 2 3 4)",
        "GEOMETRYCOLLECTION(POINT M(1 2 3), POINT(4 5 6))",
        "GEOMETRYCOLLECTION(POINT(4 5 6), POINT M(1 2 3))",
    ] {
        let expr = parse(&format!("s_intersects(geom, {wkt})"));
        let text = expr.to_text().expect("renders as text");
        // Whitespace inside a geometry is collapsed, so the ordinates are compared rather than the
        // spelling: every number written must still be there.
        let ordinates: Vec<&str> = wkt
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .collect();
        for ordinate in &ordinates {
            assert!(
                text.contains(ordinate),
                "{wkt} rendered as {text}, which lost the ordinate {ordinate}"
            );
        }
        assert!(
            text.to_uppercase().contains("M("),
            "{wkt} rendered as {text}, which lost the measure marker"
        );
    }
}

/// Generated SQL must parse as SQL, and must survive a parse/print round trip unchanged.
///
/// This checks that the emitted string is well-formed and stable. It does not detect a dropped
/// parenthesis: sqlparser prints a binary operator as `{left} {op} {right}` and only `Nested` emits
/// parentheses, so a reassociated tree prints identically. Grouping is covered semantically by
/// `tests/sql_precedence.rs`, which evaluates the SQL in DuckDB.
#[test]
fn generated_sql_round_trips_through_a_sql_parser() {
    let dialect = PostgreSqlDialect {};
    let mut cases: Vec<(String, String)> = GROUPING_BATTERY
        .iter()
        .map(|q| ((*q).to_string(), (*q).to_string()))
        .collect();
    cases.extend(example_sources("examples/text", "txt"));

    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, source) in &cases {
        // Not every CQL2 construct has a SQL rendering; those that do must round trip.
        let Ok(sql) = parse(source).to_sql() else {
            continue;
        };
        checked += 1;
        match Parser::new(&dialect)
            .try_with_sql(&sql)
            .and_then(|mut parser| parser.parse_expr())
        {
            Ok(reparsed) if reparsed.to_string() == sql => {}
            Ok(reparsed) => failures.push(format!(
                "{name}\n     sql: {sql}\n     reparsed: {reparsed}"
            )),
            Err(e) => failures.push(format!("{name}\n     sql: {sql}\n     error: {e}")),
        }
    }

    let expected_at_least = GROUPING_BATTERY.len() + 100;
    assert!(
        checked >= expected_at_least,
        "only {checked} expressions produced SQL, expected at least {expected_at_least} \
         (the battery plus the example corpus)"
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} SQL renderings changed shape when reparsed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// `tests/expected` records one file per example per encoding; every one must be readable and hold
/// the three lines the fixture format promises.
#[test]
fn golden_files_have_the_documented_shape() {
    let dir = Path::new("tests/expected");
    let mut count = 0;
    for entry in fs::read_dir(dir).expect("tests/expected should exist") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("out") {
            continue;
        }
        count += 1;
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let lines: Vec<&str> = contents.lines().collect();
        assert!(
            lines.len() >= 3,
            "{} has {} lines, expected input/text/json",
            path.display(),
            lines.len()
        );
    }
    assert!(count > 200, "expected ~229 golden files, found {count}");
}

/// A spelling the grammar accepts but the schema does not define is rewritten to the operator the
/// schema names, so it never reaches either encoding's output.
///
/// `!=` is the only such spelling: `NotEq = { "<>" | "!=" }`. Names like `eq`, `lt` and `ne` are
/// *not* aliased — the schema defines no such operators, so they can only arrive as user-defined
/// function names, which CQL2 permits. Rewriting one would silently reinterpret a valid function
/// call and, since an operator has fixed arity, corrupt it.
#[test]
fn undefined_operator_spellings_normalize() {
    for source in [
        "intfield != 5",
        r#"{"op":"!=","args":[{"property":"intfield"},5]}"#,
    ] {
        let expr = parse(source);
        assert_eq!(
            shape(&expr)["op"],
            Value::String("<>".to_string()),
            "{source} kept an undefined operator name"
        );
        let text = expr.to_text().expect("expression renders as text");
        assert!(!text.contains("!="), "{source} rendered as {text}");
        assert_eq!(
            shape(&parse(&text)),
            shape(&expr),
            "{text} did not re-parse"
        );
    }
}

/// Names the specification does not define are function names, and survive unchanged.
///
/// `div` is a genuine CQL2 operator (integer division, distinct from `/`) and is likewise left
/// alone. Each of these renders as a call in both encodings and keeps its argument count.
#[test]
fn undefined_names_are_left_as_function_calls() {
    for (name, arity) in [
        ("eq", 2),
        ("lt", 2),
        ("ne", 2),
        ("gte", 2),
        ("div", 2),
        ("eq", 3),
    ] {
        let args: Vec<String> = (0..arity).map(|i| format!("arg{i}")).collect();
        let source = format!("{name}({})", args.join(", "));
        let expr = parse(&source);
        assert_eq!(
            shape(&expr)["op"],
            Value::String(name.to_string()),
            "{source} was rewritten to an operator"
        );
        assert_eq!(
            shape(&expr)["args"].as_array().map(Vec::len),
            Some(arity),
            "{source} lost or gained arguments"
        );
        assert_eq!(
            shape(&parse(&expr.to_text().expect("renders as text"))),
            shape(&expr),
            "{source} did not round-trip"
        );
    }
}

/// `div` renders as a call, so it demands nothing of its operands.
///
/// The precedence table classifies an operator by the shape it renders in, and neither renderer
/// emits `div` infix: `to_text` falls through its arithmetic arm to the function-call fallback and
/// the SQL backend has no `div` arm either, so both write `div(a, b)`. An operand of a `div` is
/// therefore already delimited by the call's own parentheses and commas, exactly as `casei`'s is,
/// and parenthesizing it says nothing.
#[test]
fn div_renders_as_a_call_around_undelimited_operands() {
    let expr = parse("div(a or b, c) = 1");
    let text = expr.to_text().expect("renders as text");
    let sql = expr.to_sql().expect("renders as SQL");
    assert_eq!(text, "div(a OR b, c) = 1");
    assert_eq!(sql, "div(a OR b, c) = 1");
    // The reference: a function call written the same way in cql2-text.
    assert_eq!(
        parse("casei(a or b) = 'x'")
            .to_text()
            .expect("renders as text"),
        "casei(a OR b) = 'x'"
    );
    assert_eq!(
        shape(&parse(&text)),
        shape(&expr),
        "{text} did not re-parse"
    );
}

/// The infix spelling `div` still parses, and renders as the call both encodings write.
#[test]
fn infix_div_still_parses_and_renders() {
    for (source, text) in [
        ("3 = foo div 2", "3 = div(foo, 2)"),
        ("5 div 2 = 2", "div(5, 2) = 2"),
        ("a div b div c = 1", "div(div(a, b), c) = 1"),
        // An operand that binds more loosely than the operators around the call still has to be
        // parenthesized where the call is *not* what delimits it.
        ("(a div b) * c = 1", "div(a, b) * c = 1"),
        ("a * (b div c) = 1", "a * div(b, c) = 1"),
        ("a + b div c = 1", "a + div(b, c) = 1"),
    ] {
        let expr = parse(source);
        let rendered = expr.to_text().expect("renders as text");
        assert_eq!(rendered, text, "{source} rendered as {rendered}");
        assert_eq!(
            expr.to_sql().expect("renders as SQL"),
            text,
            "{source} renders differently as SQL"
        );
        assert_eq!(
            shape(&parse(&rendered)),
            shape(&expr),
            "{source} did not round-trip"
        );
    }
}

/// A property name and a function name are written the same way, and only quoted where the
/// cql2-text grammar requires it.
///
/// A cql2-text identifier is case-sensitive and admits `_`, `.` and `:`, so `Foo`, `foo.Bar` and
/// `landsat:scene_id` are names the grammar reads bare. PostgreSQL's rules, which quote anything not
/// lowercase, describe a different language, and applying them to one of the two made `MyFunc(a)`
/// and `"MyFunc" = 1` two spellings of one name.
#[test]
fn identifiers_are_quoted_only_where_the_grammar_requires() {
    const NAMES: &[&str] = &[
        "foo",
        "Foo",
        "foo.Bar",
        "landsat:scene_id",
        "MyFunc",
        "select",
        "a1",
        "and",
        "between",
        "div",
    ];

    for name in NAMES {
        let property = Expr::Property {
            property: name.to_string(),
        };
        let text = property.to_text().expect("renders as text");
        assert_eq!(text, *name, "{name} as a property name");
        assert_eq!(
            shape(&parse(&format!("{text} = 1"))),
            shape(&Expr::Operation {
                op: "=".to_string(),
                args: vec![Box::new(property), Box::new(Expr::Float(1.0))],
            }),
            "{text} did not re-parse as a property"
        );
    }

    // A function name is rendered by the same rule, so the two agree. Only names CQL2 does not
    // define are function names; the operators above are excluded because they render as themselves.
    for name in NAMES.iter().take(7) {
        let text = parse(&format!("{name}(a) = 1"))
            .to_text()
            .expect("renders as text");
        assert_eq!(text, format!("{name}(a) = 1"), "{name} as a function name");
    }
}

/// A name the grammar would read as something else keeps its quotes, and round-trips.
///
/// `Literal` is tried before `Identifier`, so a bare `true`, `false` or `null` parses as that value;
/// `not` is taken as the prefix operator and swallows the expression it precedes.
#[test]
fn reserved_names_survive_as_properties() {
    for name in ["true", "false", "null", "not", "TRUE", "Not"] {
        let expr = Expr::Operation {
            op: "=".to_string(),
            args: vec![
                Box::new(Expr::Property {
                    property: name.to_string(),
                }),
                Box::new(Expr::Float(1.0)),
            ],
        };
        let text = expr.to_text().expect("renders as text");
        assert_eq!(text, format!("\"{name}\" = 1"), "{name} lost its quotes");
        assert_eq!(
            shape(&parse(&text)),
            shape(&expr),
            "{text} did not re-parse"
        );
    }
}

/// `examples/examples.toml` records the same expected output as each example's `.txt.out` golden.
///
/// It is a documentation artifact that no other test reads, so nothing stopped it from drifting
/// away from the golden files it duplicates. Two records of one expected value that disagree is the
/// defect this suite exists to catch, so they are required to match.
#[test]
fn examples_toml_agrees_with_the_golden_files() {
    let toml = fs::read_to_string("examples/examples.toml").expect("examples.toml is present");

    let mut key = String::new();
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for line in toml.lines() {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            key = name.to_string();
            continue;
        }
        let Some((field, quoted)) = line.split_once(" = ") else {
            continue;
        };
        let line_number = match field {
            "expected_text" => 1,
            "expected_json" => 2,
            _ => continue,
        };

        let recorded = quoted
            .strip_prefix(r#"""""#)
            .and_then(|v| v.strip_suffix(r#"""""#))
            .or_else(|| quoted.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
            .unwrap_or_else(|| panic!("{key}.{field} is not a TOML string"));

        let golden = fs::read_to_string(format!("tests/expected/{key}.txt.out"))
            .unwrap_or_else(|_| panic!("{key} has no golden file"));
        let expected = golden.lines().nth(line_number).unwrap_or_default();
        if recorded != expected {
            mismatches.push(format!(
                "{key}.{field}\n    toml:   {recorded}\n    golden: {expected}"
            ));
        }
        checked += 1;
    }

    assert!(
        checked > 200,
        "expected two recorded values per example, checked {checked}"
    );
    assert!(
        mismatches.is_empty(),
        "{} recorded values disagree with tests/expected:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

/// Every route into an `Expr` normalizes, not only the two parsing functions.
///
/// `TryFrom<Value>` is how the bindings build an expression from a mapping rather than from text.
/// If it skipped normalization, the same expression would carry a different operator spelling
/// depending on which constructor produced it, `to_json` would emit a name the schema rejects, and
/// `PartialEq` would depend on provenance.
#[test]
fn building_from_a_value_normalizes() {
    const CASES: [&str; 3] = [
        // A spelling the schema does not define.
        r#"{"op":"t_metby","args":[{"interval":["2020-01-01T00:00:00Z","2020-01-02T00:00:00Z"]},{"interval":["2020-01-02T00:00:00Z","2020-01-03T00:00:00Z"]}]}"#,
        // A nested same-op chain, which the text encoding writes flat: three args, not two.
        r#"{"op":"and","args":[{"op":"=","args":[{"property":"a"},1]},{"op":"and","args":[{"op":"=","args":[{"property":"b"},2]},{"op":"=","args":[{"property":"c"},3]}]}]}"#,
        // A timestamp that is not in canonical form.
        r#"{"op":"=","args":[{"property":"a"},{"timestamp":"2012-08-10T05:30:00.000000Z"}]}"#,
    ];

    // The expected normal form, stated outright rather than by comparing two paths that could
    // both be wrong.
    const NORMALIZED: [&str; 3] = ["t_metBy", "and", "="];

    for (source, expected_op) in CASES.into_iter().zip(NORMALIZED) {
        let value: Value = serde_json::from_str(source).expect("case is valid JSON");
        let from_value = Expr::try_from(value).expect("value converts to an expression");
        assert_eq!(
            shape(&from_value)["op"],
            Value::String(expected_op.to_string()),
            "TryFrom<Value> did not normalize {source}"
        );
        let from_text = parse(source);
        assert_eq!(
            shape(&from_value),
            shape(&from_text),
            "TryFrom<Value> and parse disagree for {source}"
        );
        assert_eq!(
            from_value.to_text().expect("renders as text"),
            from_text.to_text().expect("renders as text"),
        );
    }
}

/// An operator name is accepted in any capitalization, from either encoding, and coerced to the one
/// the schema defines.
///
/// cql2-json fixes the capitalization of every operator, so a document written with a different one
/// is strictly non-conforming. It is accepted anyway and corrected: what this crate emits is held to
/// the specification, what it reads is not. A name the specification does not define is a
/// user-supplied function name and keeps the case the author wrote, from either encoding.
#[test]
fn operator_capitalization_is_coerced_from_both_encodings() {
    const OPERATORS: [(&str, &str); 6] = [
        ("T_METBY", "t_metBy"),
        ("A_CONTAINEDBY", "a_containedBy"),
        ("S_Intersects", "s_intersects"),
        ("ISNULL", "isNull"),
        ("AND", "and"),
        ("t_metby", "t_metBy"),
    ];

    for (written, canonical) in OPERATORS {
        // The two encodings are separate readers, so each is checked.
        let from_json = parse(&format!(
            r#"{{"op":"{written}","args":[{{"property":"a"}},{{"property":"b"}}]}}"#
        ));
        let from_text = parse(&format!("{written}(a, b)"));
        for (source, expr) in [("cql2-json", &from_json), ("cql2-text", &from_text)] {
            assert_eq!(
                shape(expr)["op"],
                Value::String(canonical.to_string()),
                "{written} was not coerced to {canonical} when read as {source}"
            );
        }
    }

    // A name the schema does not define is a function name, and its case is the author's.
    for source in [r#"{"op":"MyFunc","args":[{"property":"a"}]}"#, "MyFunc(a)"] {
        assert_eq!(
            shape(&parse(source))["op"],
            Value::String("MyFunc".to_string()),
            "{source} lost the author's capitalization"
        );
    }
}

/// Every operator the cql2-text grammar spells as a word is accepted in any capitalization.
///
/// The grammar writes its keywords case-insensitively, and an operator name is not part of a
/// user's data — `DIV`, `Div` and `div` are the same operator. Each case here is compared against
/// the all-lowercase spelling, so the test states the property rather than a table of outputs.
#[test]
fn keyword_operators_parse_case_insensitively() {
    const KEYWORD_FORMS: [&str; 10] = [
        "a {} b",       // and / or
        "3 = foo {} 2", // div
        "a {} 'x%'",    // like
        "a {} (1, 2)",  // in
        "a {} 1 and 2", // between
        "{}(a)",        // isNull / casei / accenti
        "{}(a, b)",     // t_metBy and friends
        "a {} b",       // repeated intentionally for the second operand set
        "{} a = 1",     // not
        "a IS {} NULL", // not, in postfix position
    ];
    const OPERATORS: [(&str, usize); 12] = [
        ("and", 0),
        ("or", 0),
        ("div", 1),
        ("like", 2),
        ("in", 3),
        ("between", 4),
        ("isNull", 5),
        ("casei", 5),
        ("accenti", 5),
        ("t_metBy", 6),
        ("not", 8),
        ("not", 9),
    ];

    let mut failures = Vec::new();
    for (operator, form) in OPERATORS {
        let lower = KEYWORD_FORMS[form].replace("{}", &operator.to_lowercase());
        let Ok(expected) = lower.parse::<Expr>() else {
            failures.push(format!("{lower} does not parse at all"));
            continue;
        };
        let expected = shape(&expected);

        for spelling in [operator.to_uppercase(), operator.to_string()] {
            let source = KEYWORD_FORMS[form].replace("{}", &spelling);
            match source.parse::<Expr>() {
                Ok(parsed) if shape(&parsed) == expected => {}
                Ok(parsed) => failures.push(format!(
                    "{source}\n     parsed as: {}\n     lowercase gives: {expected}",
                    shape(&parsed)
                )),
                Err(e) => failures.push(format!("{source} did not parse: {e}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} keyword operators are case-sensitive:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Every serde entry point normalizes, not just [`cql2::parse_json`].
///
/// `Expr` hand-writes `Deserialize` for this reason. The derived impl kept whatever the caller
/// wrote, so an `Expr` reached through serde rather than through `parse_json` — a `filter` field on
/// a search body, an element of a collection, a mapping handed to the bindings — held operator
/// spellings the cql2-json schema rejects, unflattened `and`/`or` chains, and uncanonicalized
/// timestamps. Two spellings of one filter then compared unequal.
#[test]
fn deserialization_normalizes_at_every_entry_point() {
    // A nested `and` to flatten, two operator spellings to canonicalize, and a timestamp whose
    // subsecond zeros are not part of the instant it denotes.
    const FILTER: &str = r#"{"op":"AND","args":[
        {"op":"and","args":[
            {"op":"=","args":[{"property":"a"},1]},
            {"op":"=","args":[{"property":"b"},2]}]},
        {"op":"t_metby","args":[
            {"property":"t"},
            {"timestamp":"2020-01-01T00:00:00.000Z"}]}]}"#;

    let canonical = shape(&cql2::parse_json(FILTER).expect("filter is valid cql2-json"));
    assert_eq!(
        canonical["op"], "and",
        "the outer operator should be canonicalized"
    );
    assert_eq!(
        canonical["args"].as_array().map(Vec::len),
        Some(3),
        "the nested `and` should have been flattened into the outer one"
    );
    assert_eq!(
        canonical["args"][2]["op"], "t_metBy",
        "the operator should take the spelling the schema defines"
    );
    assert_eq!(
        canonical["args"][2]["args"][1]["timestamp"], "2020-01-01T00:00:00Z",
        "the timestamp should be canonicalized"
    );

    // Reached as a struct field, the way a downstream crate holds a filter...
    #[derive(serde::Deserialize)]
    struct Search {
        filter: Expr,
    }
    let body = format!(r#"{{"filter":{FILTER}}}"#);
    let search: Search = serde_json::from_str(&body).expect("search body deserializes");
    assert_eq!(shape(&search.filter), canonical);

    // ...as an element of a collection...
    let collected: Vec<Expr> =
        serde_json::from_str(&format!("[{FILTER}]")).expect("array of filters deserializes");
    assert_eq!(shape(&collected[0]), canonical);

    // ...and through `Value`, which is how the bindings build an expression from a mapping.
    let value: Value = serde_json::from_str(FILTER).expect("filter is valid JSON");
    let converted = Expr::try_from(value).expect("value converts");
    assert_eq!(shape(&converted), canonical);
}

/// Serialization is unchanged by the hand-written impls: `Expr` still round trips through its own
/// JSON, and a normalized expression is a fixed point of another deserialization.
#[test]
fn serialization_round_trips_through_the_hand_written_impls() {
    for source in GROUPING_BATTERY {
        let expr = parse(source);
        let json = expr.to_json().expect("expression serializes");
        let reparsed: Expr = serde_json::from_str(&json).expect("its own JSON deserializes");
        assert_eq!(
            shape(&reparsed),
            shape(&expr),
            "{source} did not round trip"
        );
    }
}
