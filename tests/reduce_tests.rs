use cql2::Expr;
use rstest::rstest;
use serde_json::{json, Value};
use std::path::Path;

fn read_lines(filename: impl AsRef<Path>) -> Vec<String> {
    std::fs::read_to_string(filename)
        .unwrap() // panic on possible file-reading errors
        .lines() // split the string into an iterator of string slices
        .map(String::from) // make each slice into a string
        .collect() // gather them together into a vector
}
fn validate_reduction(a: String, b: String) {
    let properties: Value = json!(
        {
            "properties": {
                "eo:cloud_cover": 10,
                "boolfalse": false,
                "booltrue": true,
                "stringfield": "string",
                "tsfield": {"timestamp": "2020-01-01 00:00:00Z"},
                "tstarr": [1,2,3]
            },
            "geometry": {"type": "Point", "coordinates": [-93.0, 45]},
            "datetime": "2020-01-01 00:00:00Z"
        }
    );
    let inexpr: Expr = a.parse().unwrap();
    let reduced = inexpr.reduce(Some(&properties)).unwrap();
    let outexpr: Expr = b.parse().unwrap();
    assert_eq!(reduced, outexpr);
}

#[rstest]
fn validate_reduce_fixtures() {
    let lines = read_lines("tests/reductions.txt");
    let a = lines.clone().into_iter().step_by(2);
    let b = lines.clone().into_iter().skip(1).step_by(2);
    let zipped = a.zip(b);
    for (a, b) in zipped {
        validate_reduction(a, b);
    }
}
