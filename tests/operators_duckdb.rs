use cql2::{Expr, ToDuckSQL};
use duckdb::{params, Connection, Result};
use serde_json::{json, Value};
use std::fs;

#[test]
fn operators_duckdb_filter() -> Result<()> {
    // Initialize in-memory DuckDB
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(r"
        SET TimeZone='UTC';
        INSTALL SPATIAL;
        LOAD SPATIAL;
        CREATE TABLE test AS SELECT * REPLACE (st_geomfromgeojson(geom) as geom) from 'tests/cql2testdata.ndjson';
    ")?;

    // Load operators tests
    let tests =
        fs::read_to_string("tests/operators_expected.txt").expect("Failed to read operators tests");
    let mut lines = tests.lines();

    while let Some(query) = lines.next() {
        let expected_line = lines
            .next()
            .unwrap_or_else(|| panic!("Missing expected output for query: {}", query));
        // Parse expression and generate WHERE clause
        let expr: Expr = query
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse query '{}'", query));
        let where_clause = expr.to_ducksql().expect("to_ducksql failed");

        // Build and execute DuckDB query on the NDJSON source
        let sql = format!(
            "select array_to_string(array_agg(intfield::text), ' ') from test where {}",
            where_clause
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        // `array_agg` over no rows is NULL, which is the empty selection.
        let ids: String = rows
            .next()?
            .expect("aggregate query always returns one row")
            .get::<_, Option<String>>(0)
            .expect("Failed to get result")
            .unwrap_or_default();
        assert_eq!(
            ids, expected_line,
            "Query '{}' returned '{}', expected '{}'",
            query, ids, expected_line
        );
    }
    Ok(())
}

/// The rows `query` selects, according to the evaluator and according to DuckDB.
///
/// Both are given the same records and the same expression, so a difference between the two is a
/// difference in what the expression means to each of them.
fn both_engines(
    conn: &Connection,
    table: &str,
    items: &[Value],
    query: &str,
) -> Result<(Vec<i64>, Vec<i64>)> {
    let expr: Expr = query
        .parse()
        .unwrap_or_else(|e| panic!("'{query}' does not parse: {e}"));

    let evaluated: Vec<i64> = expr
        .filter(items)
        .expect("the evaluator runs the filter")
        .iter()
        .map(|item| item["id"].as_i64().expect("every record has an id"))
        .collect();

    let where_clause = expr.to_ducksql().expect("expression renders as DuckDB SQL");
    let sql = format!("select id from {table} where {where_clause} order by id");
    let mut statement = conn
        .prepare(&sql)
        .unwrap_or_else(|e| panic!("DuckDB rejected '{sql}': {e}"));
    let queried: Vec<i64> = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<_>>()?;

    Ok((evaluated, queried))
}

/// A `LIKE` pattern means the same thing to the evaluator and to DuckDB.
///
/// The evaluator's matcher escapes with `\`, as PostgreSQL does. DuckDB has no default escape
/// character, so a pattern reaches it meaning something else entirely unless the escape is stated:
/// `'item\_1'` selects `item_1` here and, without an `ESCAPE` clause, nothing at all there.
#[test]
fn like_reads_one_pattern_in_both_engines() -> Result<()> {
    // A backslash, a percent and an underscore all appear in the data, so an escape that is not
    // honoured selects visibly different rows rather than merely fewer.
    const ROWS: [(i64, &str); 6] = [
        (1, "item_1"),
        (2, "itemX1"),
        (3, "item%1"),
        (4, r"item\1"),
        (5, "itemq1"),
        (6, "item_2"),
    ];

    let conn = Connection::open_in_memory()?;
    conn.execute_batch("CREATE TABLE strings (id BIGINT, textfield VARCHAR)")?;
    for (id, text) in ROWS {
        // Bound as a parameter, so the value in the table is the value written here.
        conn.execute("INSERT INTO strings VALUES (?, ?)", params![id, text])?;
    }
    let items: Vec<Value> = ROWS
        .iter()
        .map(|(id, text)| json!({"id": id, "textfield": text}))
        .collect();

    for (query, expected) in [
        // An escaped wildcard is the literal character.
        (r"like(textfield, 'item\_1')", vec![1]),
        (r"like(textfield, 'item\%1')", vec![3]),
        // An escaped backslash is a literal backslash.
        (r"like(textfield, 'item\\1')", vec![4]),
        // An unescaped wildcard is still a wildcard: `_` is one character, `%` is any run.
        ("like(textfield, 'item_1')", vec![1, 2, 3, 4, 5]),
        ("like(textfield, 'item%')", vec![1, 2, 3, 4, 5, 6]),
    ] {
        let (evaluated, queried) = both_engines(&conn, "strings", &items, query)?;
        assert_eq!(evaluated, expected, "the evaluator disagrees on {query}");
        assert_eq!(queried, expected, "DuckDB disagrees on {query}");
    }
    Ok(())
}

/// `a_equals` compares two arrays as sets in both engines.
///
/// The evaluator collects each operand into a `HashSet`, so neither order nor a repeated element
/// changes the answer. SQL's `=` on arrays is positional, so rendering `a_equals` as `=` made
/// `a_equals(intarrayfield, (3,2,1))` true here and false in DuckDB for a row holding `[1,2,3]`.
#[test]
fn a_equals_is_set_equality_in_both_engines() -> Result<()> {
    const ROWS: [(i64, &str); 4] = [
        (1, "[1, 2, 3]"),
        // The same set, written in another order, and with a repeat.
        (2, "[3, 2, 1]"),
        (3, "[1, 2, 2, 3]"),
        // A different set.
        (4, "[1, 2]"),
    ];

    let conn = Connection::open_in_memory()?;
    conn.execute_batch("CREATE TABLE arrays (id BIGINT, intarrayfield BIGINT[])")?;
    for (id, elements) in ROWS {
        conn.execute_batch(&format!("INSERT INTO arrays VALUES ({id}, {elements})"))?;
    }
    let items: Vec<Value> = ROWS
        .iter()
        .map(|(id, elements)| {
            json!({"id": id, "intarrayfield": serde_json::from_str::<Value>(elements).unwrap()})
        })
        .collect();

    for (query, expected) in [
        ("a_equals(intarrayfield, (3, 2, 1))", vec![1, 2, 3]),
        ("a_equals((3, 2, 1), intarrayfield)", vec![1, 2, 3]),
        ("a_equals(intarrayfield, (1, 2, 2, 3))", vec![1, 2, 3]),
        ("a_equals(intarrayfield, (1, 2))", vec![4]),
        ("a_equals(intarrayfield, (1, 2, 3, 4))", vec![]),
        // The other array predicates are unchanged, and still agree.
        ("a_contains(intarrayfield, (2, 3))", vec![1, 2, 3]),
        ("a_containedby(intarrayfield, (1, 2, 3))", vec![1, 2, 3, 4]),
        ("a_overlaps(intarrayfield, (3, 9))", vec![1, 2, 3]),
    ] {
        let (evaluated, queried) = both_engines(&conn, "arrays", &items, query)?;
        assert_eq!(evaluated, expected, "the evaluator disagrees on {query}");
        assert_eq!(queried, expected, "DuckDB disagrees on {query}");
    }
    Ok(())
}
