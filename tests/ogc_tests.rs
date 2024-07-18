use cql2_rs::{parse,Validator};
use std::fs;
use std::path::PathBuf;
use rstest::rstest;




pub fn validate_file(f: &str){
    //println!("Current Directory: {:#?}", env::current_dir());
    println!("File Path: {:#?}", f);
    let cql2 = fs::read_to_string(f).unwrap();
    println!("CQL2: {}", cql2);
    let expr: cql2_rs::Expr = parse(&cql2);
    println!("Expr: {}", expr.as_json_pretty());
    let valid = expr.validate();
    assert!(valid)
}

// #[test]
// fn validate_text(){
//     println!("{:#?}", env::current_dir());
//     validate_file("ogcapi-features/cql2/standard/schema/examples/json/example01.json");
// }

#[rstest]
fn json_examples_are_valid(#[files("ogcapi-features/cql2/standard/schema/examples/json/*.json")] path: PathBuf){
    let cql2 = fs::read_to_string(path).unwrap();
    let validator = Validator::new();
    let result = validator.validate_str(&cql2);
    assert!(result)
}

#[rstest]
fn for_each_text_file(#[files("ogcapi-features/cql2/standard/schema/examples/text/*.txt")] path: PathBuf){
    validate_file(path.to_str().expect("reason"));
}

#[rstest]
fn for_each_json_file(#[files("ogcapi-features/cql2/standard/schema/examples/json/*.json")] path: PathBuf){
    validate_file(path.to_str().expect("reason"));
}

// #[test]
// fn geom_operation() {
//     let expr = parse("S_Within(Point(0 0),geom)");
//     assert_eq!(expr.as_json(), "{\"op\":\"s_within\",\"args\":[{\"coordinates\":[0.0,0.0],\"type\":\"Point\"},{\"property\":\"geom\"}]}");
// }
