use assert_json_diff::assert_json_eq;
use cql2::Expr;
use rstest::rstest;
use serde_json::json;
use std::path::{Path, PathBuf};

fn validate_str(s: &str) -> Expr {
    let expr: Expr = s.parse().unwrap();
    assert!(expr.is_valid());
    let expr_from_txt: Expr = expr.to_text().unwrap().parse().unwrap();
    assert!(expr_from_txt.is_valid());
    let json = expr.to_json().unwrap();
    let expr_from_json: Expr = json.parse().unwrap();
    assert_json_eq!(json!(json), json!(expr_from_json.to_json().unwrap()));
    expr
}

fn validate_example_path(path: impl AsRef<Path>) {
    let path = path.as_ref();
    validate_str(&std::fs::read_to_string(path).unwrap());
}

fn read_lines(filename: impl AsRef<Path>) -> Vec<String> {
    std::fs::read_to_string(filename)
        .unwrap() // panic on possible file-reading errors
        .lines() // split the string into an iterator of string slices
        .map(String::from) // make each slice into a string
        .collect() // gather them together into a vector
}

fn validate_expected_path(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let lines = read_lines(path);

    let input = &lines[0];
    let outtext = &lines[1];
    let outjson = &lines[2];

    let expr = validate_str(input);

    assert_eq!(*outtext, expr.to_text().unwrap());
    assert_eq!(*outjson, expr.to_json().unwrap());
}

#[rstest]
fn validate_text_fixtures(#[files("examples/text/*.txt")] path: PathBuf) {
    validate_example_path(path);
}

#[rstest]
fn validate_json_fixtures(#[files("examples/json/*.json")] path: PathBuf) {
    validate_example_path(path);
}

// Expected tests should have three lines.
// Line 1: input in text or json.
// Line 2: expected output in text.
// Line 3: expected output in json.
#[rstest]
fn validate_expected(#[files("tests/expected/*.out")] path: PathBuf) {
    validate_expected_path(path);
}
