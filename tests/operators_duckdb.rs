use cql2::{Expr, ToDuckSQL};
use duckdb::{Connection, Result};
use std::fs;

#[test]
fn operators_duckdb_filter() -> Result<()> {
    // Initialize in-memory DuckDB
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(r"
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
        let ids: String = rows
            .next()?
            .expect("No data returned")
            .get::<_, String>(0)
            .expect("Failed to get result");
        assert_eq!(
            ids, expected_line,
            "Query '{}' returned '{}', expected '{}'",
            query, ids, expected_line
        );
    }
    Ok(())
}
