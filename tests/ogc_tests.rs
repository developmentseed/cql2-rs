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

fn validate_path(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let file_name = path.file_name().unwrap();
    let expr = validate_str(&std::fs::read_to_string(path).unwrap());

    let input_format = path.parent().unwrap().file_stem().unwrap();
    let expected = Path::new("tests/expected").join(input_format);

    let json = std::fs::read_to_string(expected.join(file_name).with_extension("json")).unwrap();
    assert_eq!(json.trim(), expr.to_json().unwrap());

    let text = std::fs::read_to_string(expected.join(file_name).with_extension("txt")).unwrap();
    assert_eq!(text.trim(), expr.to_text().unwrap());
}

#[rstest]
fn validate_text_fixtures(#[files("fixtures/text/*.txt")] path: PathBuf) {
    validate_path(path);
}

#[rstest]
fn validate_json_fixtures(#[files("fixtures/json/*.json")] path: PathBuf) {
    validate_path(path);
}
