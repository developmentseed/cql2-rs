use cql2_rs::{parse, Validator};
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

pub fn validate_str(cql2: &str) {
    println!("CQL2: {}", cql2);
    let expr: cql2_rs::Expr = parse(&cql2);
    println!("Expr: {}", expr.as_json_pretty());
    let valid = expr.validate();
    assert!(valid)
}

pub fn validate_file(f: &str) {
    //println!("Current Directory: {:#?}", env::current_dir());
    println!("File Path: {:#?}", f);
    let cql2 = fs::read_to_string(f).unwrap();
    validate_str(&cql2)
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

#[rstest]
fn between_tests() {
    validate_str("true and a between 1 and 2 and false")
}
