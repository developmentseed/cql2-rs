use cql2::{Expr, ToDuckSQL};
use duckdb::{Connection, Result};
use std::fs;

mod geoparquet_fixture;

#[test]
fn operators_duckdb_filter() -> Result<()> {
    let geoparquet = geoparquet_fixture::geoparquet_fixture_path();
    let geoparquet = geoparquet
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "''");

    // Initialize in-memory DuckDB
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(&format!(
        r"
        INSTALL SPATIAL;
        LOAD SPATIAL;
        CREATE TABLE test AS SELECT * FROM '{}';
    ",
        geoparquet
    ))?;

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
        let where_clause = expr
            .to_ducksql()
            .expect("to_ducksql failed")
            .replace("TIMESTAMP WITH TIME ZONE", "TIMESTAMP");

        // Build and execute DuckDB query on the GeoParquet fixture
        let sql = format!(
            "select coalesce(array_to_string(array_agg(intfield::text), ' '), '') from test where {}",
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
