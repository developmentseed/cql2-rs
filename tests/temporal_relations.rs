//! Properties of the temporal comparison functions.
//!
//! OGC 21-065r2 clause 7.8 defines each relation by normative reference to the W3C/OGC Time
//! Ontology. Every relation is a condition on the four endpoints of two ranges, so each one is
//! checked here against that condition directly, over a dense set of pairs. An implementation that
//! drifts from its definition shows up immediately, whichever direction it drifts.
//!
//! An instant is a range whose bounds coincide, so it is covered by the same enumeration rather
//! than by a rule of its own.

use cql2::{Expr, ToSqlAst};

fn interval(start: u32, end: u32) -> String {
    format!("INTERVAL('2020-01-{start:02}T00:00:00Z','2020-01-{end:02}T00:00:00Z')")
}

/// Evaluates a relation over two literal ranges.
fn holds(relation: &str, left: (u32, u32), right: (u32, u32)) -> bool {
    let source = format!(
        "{relation}({}, {})",
        interval(left.0, left.1),
        interval(right.0, right.1)
    );
    let expr: Expr = source
        .parse()
        .unwrap_or_else(|e| panic!("{source} should parse: {e}"));
    match expr.reduce(None) {
        Ok(Expr::Bool(value)) => value,
        other => panic!("{source} should reduce to a boolean, got {other:?}"),
    }
}

/// Ranges with `start <= end`, including the degenerate ones, which are instants.
fn ranges() -> Vec<(u32, u32)> {
    (1..=6)
        .flat_map(|start| (start..=6).map(move |end| (start, end)))
        .collect()
}

fn pairs() -> Vec<((u32, u32), (u32, u32))> {
    let ranges = ranges();
    ranges
        .iter()
        .flat_map(|left| ranges.iter().map(move |right| (*left, *right)))
        .collect()
}

/// The condition on endpoints that defines a relation.
///
/// `l0`/`l1` are the left range's bounds and `r0`/`r1` the right's.
type Definition = fn(u32, u32, u32, u32) -> bool;

/// Each relation, with its definition.
const DEFINITIONS: [(&str, Definition); 15] = [
    ("t_before", |_l0, l1, r0, _r1| l1 < r0),
    ("t_after", |l0, _l1, _r0, r1| r1 < l0),
    ("t_meets", |_l0, l1, r0, _r1| l1 == r0),
    ("t_metBy", |l0, _l1, _r0, r1| r1 == l0),
    ("t_starts", |l0, l1, r0, r1| l0 == r0 && l1 < r1),
    ("t_startedBy", |l0, l1, r0, r1| l0 == r0 && r1 < l1),
    ("t_finishes", |l0, l1, r0, r1| r0 < l0 && l1 == r1),
    ("t_finishedBy", |l0, l1, r0, r1| l0 < r0 && l1 == r1),
    ("t_during", |l0, l1, r0, r1| r0 < l0 && l1 < r1),
    ("t_contains", |l0, l1, r0, r1| l0 < r0 && r1 < l1),
    ("t_equals", |l0, l1, r0, r1| l0 == r0 && l1 == r1),
    ("t_overlaps", |l0, l1, r0, r1| l0 < r0 && r0 < l1 && l1 < r1),
    ("t_overlappedBy", |l0, l1, r0, r1| {
        r0 < l0 && l0 < r1 && r1 < l1
    }),
    ("t_disjoint", |l0, l1, r0, r1| l1 < r0 || r1 < l0),
    ("t_intersects", |l0, l1, r0, r1| !(l1 < r0 || r1 < l0)),
];

/// Every relation agrees with its definition, on every pair of ranges.
#[test]
fn relations_match_their_definitions() {
    let pairs = pairs();
    assert!(
        pairs.len() > 400,
        "expected a dense pairing, got {}",
        pairs.len()
    );

    let mut failures = Vec::new();
    for (relation, defined) in DEFINITIONS {
        for (left, right) in &pairs {
            let expected = defined(left.0, left.1, right.0, right.1);
            let actual = holds(relation, *left, *right);
            if actual != expected {
                failures.push(format!(
                    "{relation}([{}-{}],[{}-{}]) = {actual}, definition says {expected}",
                    left.0, left.1, right.0, right.1
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} results disagree with their definition:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Each relation is the converse of its partner: `x R y` exactly when `y R' x`.
#[test]
fn converse_relations_agree() {
    const CONVERSES: [(&str, &str); 6] = [
        ("t_before", "t_after"),
        ("t_meets", "t_metBy"),
        ("t_overlaps", "t_overlappedBy"),
        ("t_starts", "t_startedBy"),
        ("t_during", "t_contains"),
        ("t_finishes", "t_finishedBy"),
    ];

    let mut failures = Vec::new();
    for (left, right) in pairs() {
        for (relation, converse) in CONVERSES {
            if holds(relation, left, right) != holds(converse, right, left) {
                failures.push(format!(
                    "{relation}([{}-{}],[{}-{}]) disagrees with its converse {converse}",
                    left.0, left.1, right.0, right.1
                ));
            }
        }
        // These three are their own converses.
        for symmetric in ["t_equals", "t_disjoint", "t_intersects"] {
            if holds(symmetric, left, right) != holds(symmetric, right, left) {
                failures.push(format!(
                    "{symmetric} is not symmetric for [{}-{}] and [{}-{}]",
                    left.0, left.1, right.0, right.1
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} converse mismatches:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// The specification derives these two from the others: `T_DISJOINT` is `before OR after`, and
/// `T_INTERSECTS` is its negation.
#[test]
fn disjoint_and_intersects_match_their_definitions() {
    let mut failures = Vec::new();
    for (left, right) in pairs() {
        let disjoint = holds("t_disjoint", left, right);
        let expected = holds("t_before", left, right) || holds("t_after", left, right);
        if disjoint != expected {
            failures.push(format!(
                "t_disjoint([{}-{}],[{}-{}]) = {disjoint}, but before-or-after = {expected}",
                left.0, left.1, right.0, right.1
            ));
        }
        if holds("t_intersects", left, right) == disjoint {
            failures.push(format!(
                "t_intersects([{}-{}],[{}-{}]) should be the negation of t_disjoint",
                left.0, left.1, right.0, right.1
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches against the derived definitions:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// An instant is the range whose bounds coincide, however it is written.
///
/// `TIMESTAMP('t')` and `INTERVAL('t','t')` denote the same range, so every relation answers the
/// same for both, and every relation accepts an instant in either operand position.
#[test]
fn an_instant_is_a_range_with_coincident_bounds() {
    const INSTANT: &str = "TIMESTAMP('2020-01-03T00:00:00Z')";
    const DEGENERATE: &str = "INTERVAL('2020-01-03T00:00:00Z','2020-01-03T00:00:00Z')";
    let other = interval(1, 5);

    let evaluate = |source: &str| -> Option<bool> {
        let expr: Expr = source.parse().ok()?;
        match expr.reduce(None) {
            Ok(Expr::Bool(value)) => Some(value),
            _ => None,
        }
    };

    let mut failures = Vec::new();
    for (relation, _) in DEFINITIONS {
        for as_left in [true, false] {
            let (left, right) = if as_left {
                (INSTANT, other.as_str())
            } else {
                (other.as_str(), INSTANT)
            };
            let (spelled_left, spelled_right) = if as_left {
                (DEGENERATE, other.as_str())
            } else {
                (other.as_str(), DEGENERATE)
            };
            let as_instant = format!("{relation}({left}, {right})");
            let spelled_out = format!("{relation}({spelled_left}, {spelled_right})");
            match (evaluate(&as_instant), evaluate(&spelled_out)) {
                (Some(a), Some(b)) if a == b => {}
                (a, b) => failures.push(format!("{as_instant} = {a:?}, {spelled_out} = {b:?}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} instants did not behave as their degenerate range:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// An operand that is not temporal at all is an error, not a temporal answer.
#[test]
fn non_temporal_operands_are_rejected() {
    for source in [
        "t_meets(1, 2)",
        "t_during(POINT(0 0), INTERVAL('2020-01-01T00:00:00Z','2020-01-05T00:00:00Z'))",
    ] {
        let expr: Expr = source.parse().expect("expression parses");
        assert!(
            expr.to_sql_ast().is_err(),
            "{source} should not render as SQL"
        );
    }
}
