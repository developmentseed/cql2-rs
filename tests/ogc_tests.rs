use cql2::{parse, Validator};
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

pub fn validate_file(f: &str) {
    //println!("Current Directory: {:#?}", env::current_dir());
    println!("File Path: {:#?}", f);
    let cql2 = fs::read_to_string(f).unwrap();
    println!("CQL2: {}", cql2);
    let expr: cql2::Expr = parse(&cql2);
    println!("Expr: {}", expr.as_json_pretty());
    let valid = expr.validate();
    assert!(valid)
}

#[rstest]
fn json_examples_are_valid(#[files("tests/fixtures/json/*.json")] path: PathBuf) {
    let cql2 = fs::read_to_string(path).unwrap();
    let validator = Validator::new();
    let result = validator.validate_str(&cql2);
    assert!(result)
}

#[rstest]
fn for_each_text_file(#[files("tests/fixtures/text/*.txt")] path: PathBuf) {
    validate_file(path.to_str().expect("reason"));
}

#[rstest]
fn for_each_json_file(#[files("tests/fixtures/json/*.json")] path: PathBuf) {
    validate_file(path.to_str().expect("reason"));
}
