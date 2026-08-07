//! Round-trip properties over generated expressions.
//!
//! The hand-written batteries elsewhere cover shapes someone thought to write down. The space of
//! expressions is unbounded, so it is sampled here instead: an expression is generated, rendered,
//! parsed back, and required to be the same expression.
//!
//! Both encodings are covered. cql2-text is where grouping can be lost, since the renderer omits
//! parentheses an operand does not need; cql2-json carries structure explicitly and mostly guards
//! the normalization both encodings share.

use cql2::Expr;
use proptest::prelude::*;

/// Properties that exist in the sample data, so generated expressions stay realistic.
const PROPERTIES: [&str; 4] = ["intfield", "floatfield", "textfield", "boolfield"];

fn scalar() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (0..PROPERTIES.len()).prop_map(|i| Expr::Property {
            property: PROPERTIES[i].to_string()
        }),
        (-1000i32..1000).prop_map(|n| Expr::Float(f64::from(n))),
        (-100i32..100).prop_map(|n| Expr::Float(f64::from(n) / 4.0)),
        "[a-z ]{0,8}".prop_map(Expr::Literal),
        any::<bool>().prop_map(Expr::Bool),
    ]
}

fn operation(op: &'static str, args: Vec<Expr>) -> Expr {
    Expr::Operation {
        op: op.to_string(),
        args: args.into_iter().map(Box::new).collect(),
    }
}

/// Arithmetic over scalars, nested a few levels deep.
fn arithmetic() -> impl Strategy<Value = Expr> {
    scalar().prop_recursive(3, 16, 2, |inner| {
        prop_oneof![(inner.clone(), inner.clone(), 0usize..5)
            .prop_map(|(l, r, which)| { operation(["+", "-", "*", "/", "%"][which], vec![l, r]) }),]
    })
}

/// A predicate: a comparison, `LIKE`, `BETWEEN` or `IS NULL` over scalars.
fn predicate() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (arithmetic(), arithmetic(), 0usize..6).prop_map(|(l, r, which)| {
            operation(["=", "<>", "<", ">", "<=", ">="][which], vec![l, r])
        }),
        (0..PROPERTIES.len(), "[a-z%_]{0,6}").prop_map(|(i, pattern)| operation(
            "like",
            vec![
                Expr::Property {
                    property: PROPERTIES[i].to_string()
                },
                Expr::Literal(pattern)
            ]
        )),
        (arithmetic(), arithmetic(), arithmetic())
            .prop_map(|(v, lo, hi)| operation("between", vec![v, lo, hi])),
        arithmetic().prop_map(|v| operation("isNull", vec![v])),
    ]
}

/// A boolean expression: predicates combined with `AND`, `OR` and `NOT`.
fn expression() -> impl Strategy<Value = Expr> {
    predicate().prop_recursive(4, 32, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 2..4).prop_map(|args| operation("and", args)),
            prop::collection::vec(inner.clone(), 2..4).prop_map(|args| operation("or", args)),
            inner.prop_map(|arg| operation("not", vec![arg])),
        ]
    })
}

/// Expressions are compared by their JSON, which is structural and independent of either rendering.
fn shape(expr: &Expr) -> String {
    expr.to_json().expect("expression serializes")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Rendering to cql2-text and parsing it back yields the same expression.
    #[test]
    fn text_rendering_round_trips(expr in expression()) {
        // Generated trees are already in the form a parse would produce, so any difference comes
        // from the rendering rather than from normalization.
        let expr = cql2::parse_json(&shape(&expr)).expect("generated expression is valid json");
        let rendered = expr.to_text().expect("expression renders as text");
        let reparsed: Expr = rendered
            .parse()
            .map_err(|e| TestCaseError::fail(format!("{rendered} did not parse: {e}")))?;
        prop_assert_eq!(shape(&reparsed), shape(&expr), "rendered as: {}", rendered);
    }

    /// The two encodings of one expression parse back to the same expression.
    ///
    /// Comparing cql2-json against itself would only show that normalization is idempotent. Parsing
    /// each rendering independently makes neither encoding able to mask a fault in the other.
    #[test]
    fn both_encodings_agree(expr in expression()) {
        let expr = cql2::parse_json(&shape(&expr)).expect("generated expression is valid json");
        let text = expr.to_text().expect("expression renders as text");
        let from_text: Expr = text
            .parse()
            .map_err(|e| TestCaseError::fail(format!("{text} did not parse: {e}")))?;
        let from_json = cql2::parse_json(&shape(&expr)).expect("rendered json parses");
        prop_assert_eq!(shape(&from_text), shape(&from_json), "rendered as: {}", text);
    }
}
