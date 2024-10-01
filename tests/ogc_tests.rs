use assert_json_diff::assert_json_eq;
use rstest::rstest;
use serde_json::json;
use std::path::{Path, PathBuf};

fn validate_path(path: impl AsRef<Path>) {
    let cql2 = std::fs::read_to_string(f).unwrap();
    let expr = cql2::parse(cql2).unwrap();
    assert!(expr.is_valid());
    let expr = cql2::parse(&expr.to_cql2_text().unwrap()).unwrap();
    assert!(expr.is_valid());
    let json = expr.to_json().unwrap();
    let expr = cql2::parse(&json).unwrap();
    assert_json_eq!(json!(json), json!(expr.to_json().unwrap()));
}

#[rstest]
fn validate_text_fixtures(#[files("tests/fixtures/text/*.txt")] path: PathBuf) {
    validate_path(path);
}

#[rstest]
fn validate_json_fixtures(#[files("tests/fixtures/json/*.json")] path: PathBuf) {
    validate_path(path);
}
