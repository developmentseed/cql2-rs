use assert_json_diff::assert_json_eq;
use cql2::{Expr, ToElasticsearch};
use std::path::Path;

fn read_lines(filename: impl AsRef<Path>) -> Vec<String> {
    std::fs::read_to_string(filename)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

/// Reads pairs of lines from `tests/elasticsearch_expected.txt` where:
/// - Line N: CQL2 input expression (text or JSON)
/// - Line N+1: expected Elasticsearch DSL compact JSON
#[test]
fn validate_elasticsearch_fixtures() {
    let lines = read_lines("tests/elasticsearch_expected.txt");
    let inputs = lines.clone().into_iter().step_by(2);
    let expecteds = lines.clone().into_iter().skip(1).step_by(2);
    for (input, expected_json_str) in inputs.zip(expecteds) {
        let expr: Expr = input
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse CQL2 '{input}': {e}"));
        let dsl = expr
            .to_elasticsearch()
            .unwrap_or_else(|e| panic!("to_elasticsearch failed for '{input}': {e}"));
        let expected: serde_json::Value = serde_json::from_str(&expected_json_str)
            .unwrap_or_else(|e| panic!("Invalid expected JSON for '{input}': {e}"));
        assert_json_eq!(dsl, expected);
    }
}
