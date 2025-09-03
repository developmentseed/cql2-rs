use cql2::Expr;
use serde_json::Value;
use std::fs;

#[test]
fn operators_expected_filter() {
    // Load test data
    let ndjson =
        fs::read_to_string("tests/cql2testdata.ndjson").expect("Failed to read NDJSON data");
    let items: Vec<Value> = ndjson
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("Invalid JSON line"))
        .collect();

    // Load expected operators tests
    let tests =
        fs::read_to_string("tests/operators_expected.txt").expect("Failed to read operators tests");
    let mut lines = tests.lines();
    while let Some(query) = lines.next() {
        let expected_line = match lines.next() {
            Some(line) => line,
            None => panic!("Missing expected output for query: {}", query),
        };
        // Parse and filter
        let expr: Expr = query
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse query '{}'", query));
        let filtered = expr.filter(&items).expect("Filter failed");
        // Collect intfield values
        let ints: Vec<String> = filtered
            .iter()
            .map(|v| {
                v.get("intfield")
                    .expect("Missing intfield")
                    .as_i64()
                    .expect("intfield not integer")
                    .to_string()
            })
            .collect();
        let result_line = ints.join(" ");
        assert_eq!(
            result_line, expected_line,
            "Query '{}' returned '{}', expected '{}'",
            query, result_line, expected_line
        );
    }
}
