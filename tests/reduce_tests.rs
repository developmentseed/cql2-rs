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

/// `reduce` must not constant-fold `IS NULL` when there is no data context,
/// because the value of the operand is unknown.
#[test]
fn is_null_not_folded_without_context() {
    let expr: Expr = "numeric IS NULL".parse().unwrap();
    let reduced = expr.reduce(None).unwrap();
    match reduced {
        Expr::Operation { op, args } => {
            assert_eq!(op, "isNull");
            assert_eq!(
                *args[0],
                Expr::Property {
                    property: "numeric".to_string()
                }
            );
        }
        other => panic!("expected the IS NULL predicate to be preserved, got {other:?}"),
    }
}

/// With a concrete record we *can* fold `IS NULL`: a present, non-null value is
/// not null, an absent field is treated as null, and literals fold regardless of
/// the data context.
#[test]
fn is_null_folds_with_known_value() {
    let present = json!({"properties": {"numeric": 5}});
    let present_null = json!({"properties": {"numeric": null}});
    let absent = json!({"properties": {"other": 5}});

    let expr: Expr = "numeric IS NULL".parse().unwrap();
    assert_eq!(expr.reduce(Some(&present)).unwrap(), Expr::Bool(false));

    let expr: Expr = "numeric IS NULL".parse().unwrap();
    assert_eq!(expr.reduce(Some(&present_null)).unwrap(), Expr::Bool(true));

    let expr: Expr = "numeric IS NULL".parse().unwrap();
    assert_eq!(expr.reduce(Some(&absent)).unwrap(), Expr::Bool(true));

    let expr: Expr = "null IS NULL".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Bool(true));

    let expr: Expr = "1 IS NULL".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Bool(false));
}

/// `reduce` must not cancel out the `IN` operator by treating the property
/// identifier as a string literal.
#[test]
fn in_not_folded_without_context() {
    let expr: Expr = "cityName IN ('Toronto','Frankfurt','Tokyo','New York')"
        .parse()
        .unwrap();
    let reduced = expr.reduce(None).unwrap();
    match reduced {
        Expr::Operation { op, args } => {
            assert_eq!(op, "in");
            assert_eq!(
                *args[0],
                Expr::Property {
                    property: "cityName".to_string()
                }
            );
        }
        other => panic!("expected the IN predicate to be preserved, got {other:?}"),
    }
}

/// `IN` still folds when every operand is a known value.
#[test]
fn in_still_folds_for_known_values() {
    let expr: Expr = "'b' IN ('a','b','c')".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Bool(true));

    let expr: Expr = "'z' IN ('a','b','c')".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Bool(false));
}

/// A negative number literal must parse to a negative literal, not be expanded
/// as `-1 * n`.
#[test]
fn negative_number_literal() {
    let expr: Expr = "property > -2".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "(property > -2)");
    match &expr {
        Expr::Operation { op, args } => {
            assert_eq!(op, ">");
            assert_eq!(*args[1], Expr::Float(-2.0));
        }
        other => panic!("expected a comparison operation, got {other:?}"),
    }

    let expr: Expr = "property > -3.14".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "(property > -3.14)");

    // Negating a non-literal (e.g. a property) is still expressed as `-1 * x`.
    let expr: Expr = "-foo".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "-1 * foo");
}
