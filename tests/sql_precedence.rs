//! Semantic tests for grouping in generated SQL.
//!
//! A SQL AST carries no grouping information, so `Display` renders binary operators without
//! parentheses and a string can be valid SQL while meaning something other than the CQL2 it came
//! from. These tests evaluate every expression twice, once with the in-Rust CQL2 evaluator and once
//! with DuckDB running the generated SQL, and require the two to select the same rows. That holds
//! regardless of how the SQL is formatted.

use cql2::{Expr, ToDuckSQL, ToSqlAst};
use duckdb::{Connection, Result};
use serde_json::Value;
use sqlparser::{dialect::PostgreSqlDialect, parser::Parser};
use std::fs;

/// Expressions whose meaning depends on grouping, each paired with the expression that
/// unparenthesized SQL would mean instead.
///
/// A grouped query can select the same rows either way, in which case it detects nothing. Each pair
/// is asserted to select *different* rows, which keeps every entry a live detector.
const PRECEDENCE_CASES: &[(&str, &str)] = &[
    // `OR` nested under `AND`, in both argument positions.
    (
        "boolfield = true and (intfield = 1 or intfield = 3)",
        "(boolfield = true and intfield = 1) or intfield = 3",
    ),
    (
        "(intfield = 1 or intfield = 2) and boolfield = true",
        "intfield = 1 or (intfield = 2 and boolfield = true)",
    ),
    (
        "(textfield = 'item_2' or textfield = 'item_3') and boolfield = false",
        "textfield = 'item_2' or (textfield = 'item_3' and boolfield = false)",
    ),
    (
        "(intfield = 1 or intfield = 2) and (intfield = 2 or intfield = 3)",
        "intfield = 1 or (intfield = 2 and intfield = 2) or intfield = 3",
    ),
    // Longer chains, where a flattened rendering reassociates.
    (
        "(intfield = 1 or intfield = 2 or intfield = 3) and boolfield = true",
        "intfield = 1 or intfield = 2 or (intfield = 3 and boolfield = true)",
    ),
    (
        "intfield < 10 and (intfield = 1 or intfield = 2) and boolfield = true",
        "(intfield < 10 and intfield = 1) or (intfield = 2 and boolfield = true)",
    ),
    // `NOT` over a compound operand.
    (
        "not (intfield = 1 or intfield = 2)",
        "not (intfield = 1) or intfield = 2",
    ),
    (
        "not (intfield > 2 and intfield < 5)",
        "not (intfield > 2) and intfield < 5",
    ),
    (
        "not (intfield = 1 or intfield = 2) and boolfield = true",
        "not (intfield = 1) or (intfield = 2 and boolfield = true)",
    ),
    (
        "boolfield = true and not (intfield = 2 or intfield = 4)",
        "(boolfield = true and not (intfield = 2)) or intfield = 4",
    ),
    // Arithmetic grouping and left-associativity.
    ("(intfield + 1) * 2 = 8", "intfield + (1 * 2) = 8"),
    ("intfield * (2 + 1) = 9", "(intfield * 2) + 1 = 9"),
    ("intfield - (2 - 1) = 1", "(intfield - 2) - 1 = 1"),
    ("intfield / (4 / 2) = 2", "(intfield / 4) / 2 = 2"),
    ("(intfield + 2) / 2 = 3", "intfield + (2 / 2) = 3"),
    (
        "intfield = (1 + 1) * 2 and boolfield = true",
        "intfield = 1 + (1 * 2) and boolfield = true",
    ),
    // Operators that would otherwise swallow a boolean operand.
    (
        "(intfield = 1 or intfield = 2) and floatfield between 1.5 and 2.5",
        "intfield = 1 or (intfield = 2 and floatfield between 1.5 and 2.5)",
    ),
    (
        "(intfield = 1 or intfield = 2) and textfield in ('item_2', 'item_3')",
        "intfield = 1 or (intfield = 2 and textfield in ('item_2', 'item_3'))",
    ),
    (
        "(intfield = 1 or intfield = 2) and textfield like 'item_2'",
        "intfield = 1 or (intfield = 2 and textfield like 'item_2')",
    ),
    (
        "(intfield = 1 or intfield = 2) and isNull(textfield)",
        "intfield = 1 or (intfield = 2 and isNull(textfield))",
    ),
    // Mixed with function-shaped operators, which are self-delimiting.
    (
        "(intfield = 1 or intfield = 2) and casei(textfield) = 'item_2'",
        "intfield = 1 or (intfield = 2 and casei(textfield) = 'item_2')",
    ),
    (
        "t_before(ts_start, DATE('2020-01-04')) and (intfield = 2 or intfield = 3)",
        "(t_before(ts_start, DATE('2020-01-04')) and intfield = 2) or intfield = 3",
    ),
];

fn test_items() -> Vec<Value> {
    fs::read_to_string("tests/cql2testdata.ndjson")
        .expect("Failed to read NDJSON data")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("Invalid JSON line"))
        .collect()
}

fn parse(query: &str) -> Expr {
    query
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse query '{}': {}", query, e))
}

/// Row identifiers selected by the in-Rust CQL2 evaluator.
fn intfields_from_filter(query: &str, items: &[Value]) -> Vec<i64> {
    let mut ints: Vec<i64> = parse(query)
        .filter(items)
        .unwrap_or_else(|e| panic!("Filter failed for '{}': {}", query, e))
        .iter()
        .map(|v| {
            v.get("intfield")
                .expect("Missing intfield")
                .as_i64()
                .expect("intfield not integer")
        })
        .collect();
    ints.sort_unstable();
    ints
}

/// Row identifiers selected by DuckDB running the generated SQL.
fn intfields_from_duckdb(conn: &Connection, query: &str) -> Result<Vec<i64>> {
    let where_clause = parse(query)
        .to_ducksql()
        .unwrap_or_else(|e| panic!("to_ducksql failed for '{}': {}", query, e));
    let sql = format!(
        "select intfield from test where {} order by 1",
        where_clause
    );
    let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| {
        panic!(
            "DuckDB rejected SQL generated for '{}': {}\n  sql: {}",
            query, e, sql
        )
    });
    stmt.query_map([], |row| row.get::<_, i64>(0))?.collect()
}

/// Renderings whose grouping is decided by the SQL precedence of an operator that is written as a
/// function call in cql2-text, so the CQL2 precedence table says nothing about it.
///
/// These are checked for agreement rather than paired, because a predicate over array or temporal
/// data has no "unparenthesized" counterpart to contrast against.
const OPERATOR_RENDERINGS: &[&str] = &[
    "a_contains(intarrayfield, (2, 3)) and (intfield = 1 or intfield = 2)",
    "(intfield = 1 or intfield = 2) and a_contains(intarrayfield, (2, 3))",
    "not a_contains(intarrayfield, (2, 3))",
    "a_containedby(intarrayfield, (1, 2, 3, 4)) and intfield < 5",
    "a_overlaps(intarrayfield, (2, 3)) or intfield = 1",
    "isNull(a_contains(intarrayfield, (2, 3)))",
    "isNull(t_disjoint(ts_start, DATE('2020-01-04')))",
    "not t_disjoint(ts_start, DATE('2020-01-04'))",
    "t_disjoint(ts_start, DATE('2020-01-04')) and intfield < 10",
    "intfield <> 5 and textfield like 'item_1'",
];

#[test]
fn ducksql_agrees_with_evaluator_on_operator_renderings() -> Result<()> {
    let conn = test_connection()?;
    let items = test_items();

    for query in OPERATOR_RENDERINGS {
        assert_eq!(
            intfields_from_duckdb(&conn, query)?,
            intfields_from_filter(query, &items),
            "DuckDB and the CQL2 evaluator disagree on '{}'\n  sql: {}",
            query,
            parse(query).to_ducksql().unwrap()
        );
    }
    Ok(())
}

fn test_connection() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        r"
        SET TimeZone='UTC';
        CREATE TABLE test AS SELECT * EXCLUDE (geom) from 'tests/cql2testdata.ndjson';
    ",
    )?;
    Ok(conn)
}

#[test]
fn ducksql_agrees_with_evaluator_on_grouped_expressions() -> Result<()> {
    let conn = test_connection()?;
    let items = test_items();

    for (grouped, flattened) in PRECEDENCE_CASES {
        let expected = intfields_from_filter(grouped, &items);
        assert_eq!(
            intfields_from_duckdb(&conn, grouped)?,
            expected,
            "DuckDB and the CQL2 evaluator disagree on '{}'\n  sql: {}",
            grouped,
            parse(grouped).to_ducksql().unwrap()
        );

        assert_ne!(
            intfields_from_filter(flattened, &items),
            expected,
            "'{}' selects the same rows as '{}', so it cannot detect a grouping error",
            grouped,
            flattened
        );
    }
    Ok(())
}

/// The generated SQL must parse as SQL, and must survive a parse/print round trip unchanged.
///
/// This checks the emitted string is well-formed and stable. Grouping is not observable here —
/// sqlparser prints a binary operator without parentheses, so a reassociated tree prints the same —
/// and is covered by the two tests above, which evaluate the SQL in DuckDB.
#[test]
fn generated_sql_round_trips_through_the_parser() {
    let dialect = PostgreSqlDialect {};
    // `operators_expected.txt` alternates query and expected-result lines.
    let operator_queries = fs::read_to_string("tests/operators_expected.txt")
        .expect("Failed to read operators tests")
        .lines()
        .step_by(2)
        .map(String::from)
        .collect::<Vec<_>>();

    let cases = PRECEDENCE_CASES
        .iter()
        .flat_map(|(grouped, flattened)| [grouped.to_string(), flattened.to_string()])
        .chain(operator_queries);

    for query in cases {
        let sql = parse(&query)
            .to_sql()
            .unwrap_or_else(|e| panic!("to_sql failed for '{}': {}", query, e));
        let reparsed = Parser::new(&dialect)
            .try_with_sql(&sql)
            .and_then(|mut parser| parser.parse_expr())
            .unwrap_or_else(|e| {
                panic!(
                    "generated SQL for '{}' does not parse: {}\n  sql: {}",
                    query, e, sql
                )
            });
        assert_eq!(
            reparsed.to_string(),
            sql,
            "generated SQL for '{}' changed shape when reparsed",
            query
        );
    }
}

/// The SQL and legacy spellings of the CQL2 predicates, in the case an author might write them.
const SPATIAL_ALIASES: &[(&str, &str)] = &[
    ("st_equals", "s_equals"),
    ("ST_Equals", "s_equals"),
    ("st_intersects", "s_intersects"),
    ("ST_Intersects", "s_intersects"),
    ("ST_INTERSECTS", "s_intersects"),
    ("intersects", "s_intersects"),
    ("INTERSECTS", "s_intersects"),
    ("Intersects", "s_intersects"),
    ("st_disjoint", "s_disjoint"),
    ("ST_DISJOINT", "s_disjoint"),
    ("st_touches", "s_touches"),
    ("St_Touches", "s_touches"),
    ("st_within", "s_within"),
    ("ST_Within", "s_within"),
    ("st_overlaps", "s_overlaps"),
    ("ST_OVERLAPS", "s_overlaps"),
    ("st_crosses", "s_crosses"),
    ("ST_Crosses", "s_crosses"),
    ("st_contains", "s_contains"),
    ("ST_CONTAINS", "s_contains"),
];

const TEMPORAL_ALIASES: &[(&str, &str)] = &[
    ("anyinteracts", "t_intersects"),
    ("AnyInteracts", "t_intersects"),
    ("ANYINTERACTS", "t_intersects"),
];

/// An alias resolves to the operator it names before anything else sees it.
///
/// The eight `st_*` names, `intersects` and `anyinteracts` are alternate spellings of operators CQL2
/// does define, so they are folded at ingress rather than in one backend: an expression that arrives
/// spelled `ST_Intersects` *is* an `s_intersects` operation by the time it has been parsed, in
/// either encoding and whatever its case.
#[test]
fn aliases_are_resolved_at_ingress() {
    for (alias, canonical) in SPATIAL_ALIASES.iter().chain(TEMPORAL_ALIASES) {
        let Expr::Operation { op, .. } = parse(&format!("{alias}(a, b)")) else {
            panic!("'{alias}(a, b)' should parse to an operation");
        };
        assert_eq!(op, *canonical, "cql2-text left '{alias}' unresolved");

        let json = format!(r#"{{"op":"{alias}","args":[{{"property":"a"}},{{"property":"b"}}]}}"#);
        let Expr::Operation { op, .. } = parse(&json) else {
            panic!("'{json}' should parse to an operation");
        };
        assert_eq!(op, *canonical, "cql2-json left '{alias}' unresolved");
    }
}

/// An alias renders as the predicate it names, in both encodings.
///
/// Each alias is checked against the CQL2 operator it stands for rather than against a literal
/// string, so the assertion says what the alias means. Literal spellings are pinned at the end so
/// the two sides cannot drift together.
#[test]
fn aliases_render_as_the_predicate_they_name() {
    let renderings = |query: &str| {
        let expr = parse(query);
        (
            expr.to_sql()
                .unwrap_or_else(|e| panic!("to_sql failed for '{query}': {e}")),
            expr.to_text()
                .unwrap_or_else(|e| panic!("to_text failed for '{query}': {e}")),
        )
    };
    for (alias, canonical) in SPATIAL_ALIASES {
        assert_eq!(
            renderings(&format!("{alias}(geom, POINT(0 0))")),
            renderings(&format!("{canonical}(geom, POINT(0 0))")),
            "'{alias}' does not render as '{canonical}'"
        );
    }
    for (alias, canonical) in TEMPORAL_ALIASES {
        assert_eq!(
            renderings(&format!("{alias}(ts_start, DATE('2020-01-04'))")),
            renderings(&format!("{canonical}(ts_start, DATE('2020-01-04'))")),
            "'{alias}' does not render as '{canonical}'"
        );
    }

    assert_eq!(
        renderings("ST_Intersects(geom, POINT(0 0))"),
        (
            "st_intersects(geom, st_geomfromtext('POINT(0 0)'))".to_string(),
            "s_intersects(geom, POINT(0 0))".to_string(),
        )
    );
}

/// The evaluator folds an aliased predicate exactly as it folds the canonical one.
///
/// This is what folding at ingress buys: `reduce` dispatches on `SPATIALOPS` and `TEMPORALOPS`,
/// which name none of the aliases, so while the aliasing lived in the SQL backend an
/// `ST_Intersects` expression rendered as SQL but would not evaluate. Each case is asserted to
/// reduce to a boolean, so an alias that stopped folding would fail here rather than agree
/// vacuously with an equally unreduced canonical form.
#[test]
fn aliases_reduce_as_the_predicate_they_name() {
    let reduced = |query: &str| {
        parse(query)
            .reduce(None)
            .unwrap_or_else(|e| panic!("reduce failed for '{query}': {e}"))
    };
    for (alias, canonical) in SPATIAL_ALIASES {
        for operand in ["POINT(0 0)", "POINT(1 1)"] {
            let value = reduced(&format!("{alias}(POINT(0 0), {operand})"));
            assert!(
                matches!(value, Expr::Bool(_)),
                "'{alias}(POINT(0 0), {operand})' did not reduce to a boolean, got {value:?}"
            );
            assert_eq!(
                value,
                reduced(&format!("{canonical}(POINT(0 0), {operand})")),
                "'{alias}' does not evaluate as '{canonical}'"
            );
        }
    }
    for (alias, canonical) in TEMPORAL_ALIASES {
        for operand in ["DATE('2020-01-01')", "DATE('2021-06-30')"] {
            let value = reduced(&format!("{alias}(DATE('2020-01-01'), {operand})"));
            assert!(
                matches!(value, Expr::Bool(_)),
                "'{alias}(DATE('2020-01-01'), {operand})' did not reduce to a boolean, got {value:?}"
            );
            assert_eq!(
                value,
                reduced(&format!("{canonical}(DATE('2020-01-01'), {operand})")),
                "'{alias}' does not evaluate as '{canonical}'"
            );
        }
    }
}

/// An n-ary arithmetic operation renders as a flat chain, folded to the left.
///
/// cql2-json can hold any number of operands — `{"op": "-", "args": [10, 3, 2]}` — and `to_text`
/// renders that as `10 - 3 - 2`. The SQL rendering has to say the same thing, and for the
/// non-commutative operators saying it means folding left: `10 - 3 - 2` is 5, not 9.
///
/// The chain is checked by arithmetic rather than by shape, because a right-folded tree prints
/// identically: sqlparser emits no parentheses of its own, so `(10 - 3) - 2` and `10 - (3 - 2)` are
/// the same string. Only the database can tell the two apart.
#[test]
fn nary_arithmetic_chains_to_the_left() {
    // Operator, operands, and the value a left fold gives, at two, three and four operands.
    const CASES: &[(&str, &[f64], f64)] = &[
        ("+", &[10.0, 3.0], 13.0),
        ("+", &[10.0, 3.0, 2.0], 15.0),
        ("+", &[10.0, 3.0, 2.0, 4.0], 19.0),
        ("-", &[10.0, 3.0], 7.0),
        ("-", &[10.0, 3.0, 2.0], 5.0),
        ("-", &[10.0, 3.0, 2.0, 4.0], 1.0),
        ("*", &[10.0, 3.0], 30.0),
        ("*", &[10.0, 3.0, 2.0], 60.0),
        ("*", &[10.0, 3.0, 2.0, 4.0], 240.0),
        ("/", &[64.0, 4.0], 16.0),
        ("/", &[64.0, 4.0, 2.0], 8.0),
        ("/", &[64.0, 4.0, 2.0, 8.0], 1.0),
        ("%", &[23.0, 10.0], 3.0),
        ("%", &[23.0, 10.0, 2.0], 1.0),
        ("%", &[23.0, 10.0, 2.0, 1.0], 0.0),
    ];

    let connection = duckdb::Connection::open_in_memory().expect("in-memory duckdb");
    for (op, operands, expected) in CASES {
        let expr = Expr::Operation {
            op: op.to_string(),
            args: operands.iter().map(|n| Box::new(Expr::Float(*n))).collect(),
        };
        let sql = expr.to_sql().expect("n-ary arithmetic renders as SQL");

        let flat = operands
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(&format!(" {op} "));
        assert_eq!(sql, flat, "{op} with {} operands", operands.len());
        assert_eq!(
            expr.to_text().expect("n-ary arithmetic renders as text"),
            sql,
            "the text and SQL renderings of {op} disagree"
        );

        let value: f64 = connection
            .query_row(&format!("SELECT {sql}"), [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("DuckDB rejected {sql}: {e}"));
        assert_eq!(value, *expected, "{sql} did not fold to the left");
    }
}

/// A single operand is rejected rather than printed with the operator dropped.
#[test]
fn arithmetic_needs_at_least_two_operands() {
    for op in ["+", "-", "*", "/", "%"] {
        let expr = Expr::Operation {
            op: op.to_string(),
            args: vec![Box::new(Expr::Float(10.0))],
        };
        assert!(
            expr.to_sql().is_err(),
            "{op} applied to one operand rendered as {:?}",
            expr.to_sql()
        );
    }
}

/// An explicitly right-nested operand keeps the parentheses that record it.
#[test]
fn right_nested_arithmetic_keeps_its_grouping() {
    let expr = Expr::Operation {
        op: "-".to_string(),
        args: vec![
            Box::new(Expr::Float(10.0)),
            Box::new(Expr::Operation {
                op: "-".to_string(),
                args: vec![Box::new(Expr::Float(3.0)), Box::new(Expr::Float(2.0))],
            }),
        ],
    };
    let sql = expr.to_sql().expect("nested arithmetic renders as SQL");
    assert_eq!(sql, "10 - (3 - 2)");

    let connection = duckdb::Connection::open_in_memory().expect("in-memory duckdb");
    let value: f64 = connection
        .query_row(&format!("SELECT {sql}"), [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("DuckDB rejected {sql}: {e}"));
    assert_eq!(value, 9.0, "{sql} lost the grouping it was given");
}

/// A string literal survives SQL generation with its value intact, whatever it contains.
///
/// The escaping is not this crate's own: values are handed to sqlparser, which prints them. That
/// makes correctness depend on how sqlparser treats a value that already looks escaped, so it is
/// pinned here by executing the generated SQL and reading the value back out of the database
/// rather than by comparing the generated text against an expected spelling.
#[test]
fn string_literals_survive_sql_generation() {
    const HOSTILE: [&str; 12] = [
        "plain",
        "O'Brien",
        "two''doubled",
        r"back\slash",
        r"backslash-then-quote\'",
        r"\' OR 1=1 --",
        "'; DROP TABLE t; --",
        "quote\"double",
        "semi;colon",
        "dash--dash",
        "slash/*star*/",
        "unicode ✓ ünïcøde",
    ];

    let connection = duckdb::Connection::open_in_memory().expect("in-memory duckdb");
    for value in HOSTILE {
        let expr: Expr = Expr::Operation {
            op: "=".to_string(),
            args: vec![
                Box::new(Expr::Literal(value.to_string())),
                Box::new(Expr::Literal(value.to_string())),
            ],
        };
        let sql = expr.to_ducksql().expect("expression renders as SQL");

        // Both sides carry the same literal, so a predicate that escapes its quoting stops being a
        // comparison of one value against itself.
        let matched: bool = connection
            .query_row(&format!("SELECT {sql}"), [], |row| row.get(0))
            .unwrap_or_else(|e| {
                panic!("{value:?} generated SQL the database rejected: {sql}\n{e}")
            });
        assert!(matched, "{value:?} did not compare equal to itself: {sql}");
    }
}

/// A non-finite number renders as a value the database reads as that number.
///
/// Written as a bare token, `inf` is an identifier: a database reads it as a column reference and
/// either errors or, worse, resolves it. Both PostgreSQL and DuckDB accept the IEEE names cast to a
/// floating-point type, which is the same spelling `..` interval bounds already use.
#[test]
fn non_finite_numbers_render_as_the_values_they_name() {
    let connection = duckdb::Connection::open_in_memory().expect("in-memory duckdb");
    for (source, expected) in [
        ("5 > 1/0", false),  // 5 > +Infinity
        ("5 > 0-1/0", true), // 5 > -Infinity
        ("5 < 1/0", true),
        ("5 > 0/0", false), // every comparison with NaN is false
        ("5 < 0/0", false),
    ] {
        let expr: Expr = source.parse().expect("expression parses");
        let sql = expr
            .reduce(None)
            .expect("expression reduces")
            .to_ducksql()
            .expect("expression renders as SQL");
        let actual: bool = connection
            .query_row(&format!("SELECT {sql}"), [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("{source} generated SQL the database rejected: {sql}\n{e}"));
        assert_eq!(actual, expected, "{source} rendered as {sql}");
    }
}
