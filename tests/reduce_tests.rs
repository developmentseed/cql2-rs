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

fn operation(op: &str, args: Vec<Expr>) -> Expr {
    Expr::Operation {
        op: op.to_string(),
        args: args.into_iter().map(Box::new).collect(),
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

/// `div` is integer division, which is what makes it an operator distinct from `/`.
///
/// The quotient is truncated toward zero, so `-5 div 2` is -2 rather than -3: that is what Rust's
/// `/` does on integers, what PostgreSQL's `/` and `div()` do, and what the SQL standard requires.
#[test]
fn div_is_integer_division() {
    const CASES: &[(f64, f64, f64)] = &[
        (5.0, 2.0, 2.0),
        (7.0, 7.0, 1.0),
        (1.0, 3.0, 0.0),
        (0.0, 5.0, 0.0),
        // Truncation toward zero, on each combination of signs.
        (-5.0, 2.0, -2.0),
        (5.0, -2.0, -2.0),
        (-5.0, -2.0, 2.0),
    ];
    for (left, right, expected) in CASES {
        let expr = operation("div", vec![Expr::Float(*left), Expr::Float(*right)]);
        assert_eq!(
            expr.clone().reduce(None).expect("div reduces"),
            Expr::Float(*expected),
            "{} is not {expected}",
            expr.to_text().expect("renders as text")
        );
    }

    // Written infix in cql2-text, the same operator.
    let expr: Expr = "5 div 2".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Float(2.0));
    // And distinct from `/`, which is exact.
    let expr: Expr = "5 / 2".parse().unwrap();
    assert_eq!(expr.reduce(None).unwrap(), Expr::Float(2.5));
}

/// Dividing by zero has no integer answer, so `div` is left unfolded rather than panicking or
/// inventing the infinity that `/` yields.
#[test]
fn div_by_zero_is_left_unfolded() {
    let expr: Expr = "5 div 0".parse().unwrap();
    let reduced = expr.reduce(None).expect("div by zero is not an error");
    assert_eq!(
        reduced,
        operation("div", vec![Expr::Float(5.0), Expr::Float(0.0)])
    );
    assert_eq!(reduced.to_text().unwrap(), "div(5, 0)");
}

/// An arithmetic operation over any number of operands folds, chained to the left.
///
/// cql2-json can hold any number of operands — `{"op": "-", "args": [10, 3, 2]}` — and both
/// renderings write that as `10 - 3 - 2`, which reads as `(10 - 3) - 2`. `reduce` has to agree, and
/// for `-` and `/` the direction of the fold is the whole of the answer: 5, not 9.
#[test]
fn nary_arithmetic_folds_to_the_left() {
    // Operator, operands, and the value a left fold gives, at two, three and four operands.
    const CASES: &[(&str, &[f64], f64)] = &[
        ("+", &[10.0, 3.0], 13.0),
        ("+", &[10.0, 3.0, 2.0], 15.0),
        ("+", &[10.0, 3.0, 2.0, 4.0], 19.0),
        ("-", &[10.0, 3.0], 7.0),
        ("-", &[10.0, 3.0, 2.0], 5.0),
        ("-", &[10.0, 3.0, 2.0, 4.0], 1.0),
        ("*", &[10.0, 3.0], 30.0),
        ("*", &[10.0, 3.0, 2.0], 60.0),
        ("*", &[10.0, 3.0, 2.0, 4.0], 240.0),
        ("/", &[64.0, 4.0], 16.0),
        ("/", &[64.0, 4.0, 2.0], 8.0),
        ("/", &[64.0, 4.0, 2.0, 8.0], 1.0),
        ("%", &[23.0, 10.0], 3.0),
        ("%", &[23.0, 10.0, 2.0], 1.0),
        ("%", &[23.0, 10.0, 2.0, 1.0], 0.0),
    ];

    for (op, operands, expected) in CASES {
        let expr = Expr::Operation {
            op: op.to_string(),
            args: operands.iter().map(|n| Box::new(Expr::Float(*n))).collect(),
        };
        assert_eq!(
            expr.clone().reduce(None).expect("n-ary arithmetic reduces"),
            Expr::Float(*expected),
            "{} did not fold to the left",
            expr.to_text().expect("renders as text")
        );
    }
}

/// An explicitly right-nested operand means what it says, and is not flattened into the chain.
#[test]
fn right_nested_arithmetic_keeps_its_grouping() {
    for (source, expected) in [
        ("10 - (3 - 2)", 9.0),
        ("10 - 3 - 2", 5.0),
        ("64 / (4 / 2)", 32.0),
        ("64 / 4 / 2", 8.0),
    ] {
        let expr: Expr = source.parse().expect("expression parses");
        assert_eq!(
            expr.reduce(None).expect("expression reduces"),
            Expr::Float(expected),
            "{source} did not evaluate as written"
        );
    }
}

/// A chain that cannot be folded keeps the flat shape it came in with.
///
/// Half of a chain folding would leave a nested operation where both renderings expect a flat one.
#[test]
fn unfoldable_arithmetic_chains_are_left_alone() {
    let expr = Expr::Operation {
        op: "+".to_string(),
        args: vec![
            Box::new(Expr::Property {
                property: "unknown".to_string(),
            }),
            Box::new(Expr::Float(3.0)),
            Box::new(Expr::Float(2.0)),
        ],
    };
    let reduced = expr.clone().reduce(None).expect("expression reduces");
    assert_eq!(reduced, expr);
    assert_eq!(reduced.to_text().unwrap(), "unknown + 3 + 2");
}

/// A negative number literal must parse to a negative literal, not be expanded
/// as `-1 * n`.
#[test]
fn negative_number_literal() {
    let expr: Expr = "property > -2".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "property > -2");
    match &expr {
        Expr::Operation { op, args } => {
            assert_eq!(op, ">");
            assert_eq!(*args[1], Expr::Float(-2.0));
        }
        other => panic!("expected a comparison operation, got {other:?}"),
    }

    let expr: Expr = "property > -3.14".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "property > -3.14");

    // Negating a non-literal (e.g. a property) is still expressed as `-1 * x`.
    let expr: Expr = "-foo".parse().unwrap();
    assert_eq!(expr.to_text().unwrap(), "-1 * foo");
}
