use cql2::{parse, Expr, Validator};
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

pub fn validate_file(f: &str) {
    println!("File Path: {:#?}", f);
    let cql2 = fs::read_to_string(f).unwrap();
    println!("CQL2: {}", cql2);
    let expr: Expr = parse(&cql2);
    println!("Expr: {}", expr.to_json_pretty().unwrap());

    Validator::new()
        .unwrap()
        .validate(&expr.to_value().unwrap())
        .unwrap();
}

#[rstest]
fn for_each_text_file(#[files("tests/fixtures/text/*.txt")] path: PathBuf) {
    validate_file(path.to_str().expect("reason"));
}

#[rstest]
fn for_each_json_file(#[files("tests/fixtures/json/*.json")] path: PathBuf) {
    validate_file(path.to_str().expect("reason"));
}
