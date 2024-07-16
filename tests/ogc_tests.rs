use cql2_rs::{parse, get_validator};
use std::env;
use std::fs;
use std::path::PathBuf;
use rstest::rstest;
use boon::{Schemas, Compiler};


pub fn boon_validate(cql2: &str)->bool{
    let mut schemas = Schemas::new(); // container for compiled schemas
    let mut compiler = Compiler::new();
    let sch_index = compiler.compile("ogcapi-features/cql2/standard/schema/cql2.json", &mut schemas).unwrap();
    let cql2_json = serde_json::from_str(&cql2).unwrap();
    let valid = schemas.validate(&cql2_json, sch_index).is_ok();
    return valid
}

pub fn validate_file(f: &str){
    //println!("Current Directory: {:#?}", env::current_dir());
    println!("File Path: {:#?}", f);
    let cql2 = fs::read_to_string(f).unwrap();
    println!("CQL2: {}", cql2);
    let expr: cql2_rs::Expr = parse(&cql2);
    println!("Expr: {}", expr.as_json_pretty());
    let valid = boon_validate(expr.as_json().as_str());
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
    let result = boon_validate(&cql2);
    assert!(result)
    // let validator = get_validator();
    // let cql2 = fs::read_to_string(path).unwrap();
    // println!("CQL2: {}", cql2);
    // let cql2_json = serde_json::from_str(&cql2).unwrap();
    // let result = validator.validate(&cql2_json);
    // if let Err(errors) = result {
    //     for error in errors {
    //         println!("Validation error: {}", error);
    //         println!("Instance path: {}", error.instance_path.to_string());
    //     }
    //     assert!(false)
    // }
    // assert!(true)
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
