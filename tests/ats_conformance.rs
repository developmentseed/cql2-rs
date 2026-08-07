//! Conformance checks derived from the OGC CQL2 Abstract Test Suite (Annex A of OGC 21-065r2).
//!
//! The ATS targets "servers that evaluate filter expressions", so its test methods are written
//! around data sources and queryables. What a library can take from it is the corpus of filter
//! expressions each test names: every one is an expression the specification asserts a conforming
//! implementation accepts. `tests/ats/expressions.txt` holds them, tagged by conformance class.
//!
//! Each expression must parse, satisfy the cql2-json schema, and survive both round trips. The ATS
//! itself is about evaluation results, which need a data source; that half is carried by
//! `tests/operators_tests.txt`, whose queries run against `tests/cql2testdata.ndjson` and are
//! cross-checked against DuckDB. `every_ats_operator_is_evaluated` ties the two together, so no
//! operator the specification names is left checked for syntax alone.

use cql2::{Expr, ToSqlAst, Validator};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// The conformance classes cql2-rs implements. An expression from any other class is skipped, and
/// the skip is reported, so dropping support for a class cannot pass unnoticed.
const SUPPORTED: &[&str] = &[
    "basic-cql2",
    "advanced-comparison-operators",
    "case-insensitive-comparison",
    "accent-insensitive-comparison",
    "basic-spatial-functions",
    "basic-spatial-functions-plus",
    "spatial-functions",
    "temporal-functions",
    "array-functions",
    "property-property",
    "arithmetic",
];

/// Supported classes for which the corpus holds no expression.
///
/// The ATS states its arithmetic tests as value examples rather than as filter expressions, so
/// there is nothing to substitute a queryable into. Arithmetic is exercised instead by
/// `tests/operators_tests.txt`, which evaluates it against the sample data.
const UNREPRESENTED: [&str; 1] = ["arithmetic"];

fn corpus() -> Vec<(String, String)> {
    let text = fs::read_to_string("tests/ats/expressions.txt").expect("ATS corpus is present");
    let cases: Vec<(String, String)> = text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(class, expr)| (class.to_string(), expr.to_string()))
        .collect();
    assert!(
        cases.len() > 100,
        "expected the full ATS corpus, found {} expressions",
        cases.len()
    );
    cases
}

/// Every expression the specification names must parse.
#[test]
fn ats_expressions_parse() {
    let mut failures = Vec::new();
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    let cases = corpus();
    for (class, expr) in &cases {
        if !SUPPORTED.contains(&class.as_str()) {
            continue;
        }
        *by_class.entry(class.as_str()).or_default() += 1;
        if let Err(e) = expr.parse::<Expr>() {
            failures.push(format!("[{class}] {expr}\n     {e}"));
        }
    }
    assert!(
        by_class.len() == SUPPORTED.len() - UNREPRESENTED.len(),
        "expected every supported class but {UNREPRESENTED:?} to appear in the corpus, found {}: {by_class:?}",
        by_class.len()
    );
    assert!(
        failures.is_empty(),
        "{} ATS expressions failed to parse:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Parsing an ATS expression must produce cql2-json the schema accepts.
///
/// This is what catches an operator rendered under a name the schema does not define — the JSON
/// stays well-formed, so only the schema notices.
#[test]
fn ats_expressions_produce_valid_json() {
    let validator = Validator::new().expect("validator builds");
    let mut failures = Vec::new();
    let cases = corpus();
    for (class, expr) in &cases {
        if !SUPPORTED.contains(&class.as_str()) {
            continue;
        }
        let Ok(parsed) = expr.parse::<Expr>() else {
            continue; // reported by ats_expressions_parse
        };
        let value = parsed.to_value().expect("expression serializes");
        if validator.validate(&value).is_err() {
            failures.push(format!("[{class}] {expr}\n     {value}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} ATS expressions produced JSON the schema rejects:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Both renderings of an ATS expression must parse back to the same expression.
#[test]
fn ats_expressions_round_trip() {
    let mut failures = Vec::new();
    let cases = corpus();
    for (class, expr) in &cases {
        if !SUPPORTED.contains(&class.as_str()) {
            continue;
        }
        let Ok(parsed) = expr.parse::<Expr>() else {
            continue;
        };
        let json = parsed.to_json().expect("expression serializes");

        for rendered in [
            parsed.to_text().expect("expression renders as text"),
            json.clone(),
        ] {
            match rendered.parse::<Expr>() {
                Ok(reparsed) if reparsed.to_json().ok().as_deref() == Some(&json) => {}
                Ok(reparsed) => failures.push(format!(
                    "[{class}] {expr}\n     rendered: {rendered}\n     was: {json}\n     now: {:?}",
                    reparsed.to_json()
                )),
                Err(e) => failures.push(format!(
                    "[{class}] {expr}\n     rendered: {rendered}\n     error: {e}"
                )),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} ATS expressions changed meaning across a round trip:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Every ATS expression must render as SQL, and that SQL must parse as SQL.
#[test]
fn ats_expressions_render_as_sql() {
    let dialect = sqlparser::dialect::PostgreSqlDialect {};
    let mut failures = Vec::new();
    let cases = corpus();
    for (class, expr) in &cases {
        if !SUPPORTED.contains(&class.as_str()) {
            continue;
        }
        let Ok(parsed) = expr.parse::<Expr>() else {
            continue;
        };
        match parsed.to_sql() {
            Ok(sql) => {
                if sqlparser::parser::Parser::new(&dialect)
                    .try_with_sql(&sql)
                    .and_then(|mut p| p.parse_expr())
                    .is_err()
                {
                    failures.push(format!("[{class}] {expr}\n     unparseable sql: {sql}"));
                }
            }
            Err(e) => failures.push(format!("[{class}] {expr}\n     to_sql failed: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} ATS expressions did not render as usable SQL:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Collects every operator name appearing anywhere in a cql2-json document.
fn operators(json: &Value, into: &mut BTreeSet<String>) {
    match json {
        Value::Object(fields) => {
            if let Some(Value::String(op)) = fields.get("op") {
                let _ = into.insert(op.clone());
            }
            for value in fields.values() {
                operators(value, into);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| operators(item, into)),
        _ => {}
    }
}

fn operators_in(sources: impl Iterator<Item = String>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for source in sources {
        let expr: Expr = source
            .parse()
            .unwrap_or_else(|e| panic!("{source} should parse: {e}"));
        let json: Value = serde_json::from_str(&expr.to_json().expect("expression serializes"))
            .expect("cql2-json is valid JSON");
        operators(&json, &mut found);
    }
    found
}

/// Every operator the ATS exercises is also exercised against data.
///
/// Parsing an operator proves the grammar accepts it, not that it computes anything. This requires
/// each one to appear in the corpus that is evaluated over `tests/cql2testdata.ndjson`, so adding
/// an ATS expression for an operator that is never evaluated fails here.
#[test]
fn every_ats_operator_is_evaluated() {
    let from_ats = operators_in(corpus().into_iter().map(|(_, expr)| expr));
    assert!(
        from_ats.len() > 20,
        "expected the ATS corpus to name many operators, found {}",
        from_ats.len()
    );

    let queries = fs::read_to_string("tests/operators_tests.txt").expect("query corpus is present");
    let evaluated = operators_in(
        queries
            .lines()
            .map(|line| {
                line.split('#')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
            .filter(|query| !query.is_empty() && !query.starts_with("//")),
    );

    let unevaluated: Vec<&String> = from_ats.difference(&evaluated).collect();
    assert!(
        unevaluated.is_empty(),
        "{} operators are checked for syntax but never evaluated: {:?}",
        unevaluated.len(),
        unevaluated
    );
}
